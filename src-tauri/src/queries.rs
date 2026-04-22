//! queries.rs — SQL query builders for HerbReady.
//!
//! All queries are pure SELECT — no DDL / DML.
//! Parameter placeholders are `?` (MySQL / sqlx style).
//!
//! The builders return `(String, Vec<String>)` where the Vec contains the
//! parameter values in positional order.  Callers bind them via sqlx.

use crate::config::DrugConfig;

// ---------------------------------------------------------------------------
// Helper: build the icode IN(...) list literal
// ---------------------------------------------------------------------------

/// Returns a comma-separated string of quoted icode literals.
/// e.g. `"'1580004','1500018','1580003'"`
pub fn build_icode_list(drugs: &[DrugConfig]) -> String {
    drugs
        .iter()
        .map(|d| format!("'{}'", d.icode))
        .collect::<Vec<_>>()
        .join(",")
}

/// Returns a `CASE icode WHEN '...' THEN N ...` fragment for course_days.
pub fn build_course_days_case(drugs: &[DrugConfig]) -> String {
    drugs
        .iter()
        .map(|d| format!("            WHEN '{}' THEN {}", d.icode, d.course_days))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Daily query  (Tab 1)
// ---------------------------------------------------------------------------

/// Build the daily-processing query.
///
/// Params order: `[process_date × 4]` (dept codes inlined into SQL).
///
/// 1. lv subquery  — vstdate < ?
/// 2. h subquery   — DATE_SUB(?, INTERVAL 1 YEAR)
/// 3. h subquery   — o.vstdate < ?
/// 4. WHERE clause — v.vstdate = ?
pub fn build_daily_query(
    process_date: &str,
    dept_codes: &[String],
    drugs: &[DrugConfig],
    vitals_on_date: bool,
) -> (String, Vec<String>) {
    let icode_list = build_icode_list(drugs);
    let course_days_case = build_course_days_case(drugs);

    let in_clause = if dept_codes.is_empty() {
        "'011'".to_string()
    } else {
        dept_codes
            .iter()
            .map(|c| format!("'{}'", c))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // ── Strategy ────────────────────────────────────────────────────────────
    // 1. Drive from ovst for the target date + department (small set, index hit).
    // 2. Use a pre-aggregated drug-history subquery (h) scoped to 1 year — this
    //    is the single most expensive part; we limit it to exactly the HNs that
    //    appear on the day via a semi-join on the inline hn_today CTE.
    // 3. Drug master (m) is tiny (≤ ~20 rows from drugitems) — CROSS JOIN is fine.
    // 4. lv (last-visit date) and ls (opdscreen vitals) are lightweight lookups.
    // 5. GROUP BY only on the natural PK (vn) + non-aggregated SELECT columns.
    //    We deliberately exclude ls.* from GROUP BY to avoid MySQL choosing a
    //    suboptimal plan; ls columns are functionally dependent on ls.vn = v.vn.
    // ────────────────────────────────────────────────────────────────────────

    // Build the vitals sub-join depending on the vitals_on_date flag.
    //
    // vitals_on_date = true  → only rows whose visit date equals process_date
    //                          (same visit as the one being processed).
    // vitals_on_date = false → latest opdscreen row per patient up to and
    //                          including process_date (legacy / default behaviour).
    //
    // NOTE: process_date and in_clause are inlined here (not bound as `?`)
    // because they are already used as literals elsewhere in the same query.
    // The vitals subquery does not add any new `?` placeholders.
    let vitals_join = if vitals_on_date {
        format!(
            r#"SELECT os.vn,
                      MAX(os.bw)  AS bw,
                      MAX(os.bps) AS bps,
                      MAX(os.bpd) AS bpd,
                      MAX(os.pulse) AS pulse
               FROM   opdscreen os
               JOIN   ovst ov ON ov.vn = os.vn
               WHERE  ov.vstdate = '{pd}'
                 AND  ov.cur_dep IN ({ic})
               GROUP  BY os.vn"#,
            pd = process_date,
            ic = in_clause,
        )
    } else {
        // Return today's VN (so the outer LEFT JOIN on vitals.vn = v.vn matches)
        // but pull vital signs from the latest PAST visit strictly before process_date.
        // Bug fixes:
        //   1. Use `< '{pd}'` (strict) so process_date vitals are never included.
        //   2. Return v_today.vn (today's VN) instead of the past visit's VN so
        //      the outer join `vitals.vn = v.vn` actually matches.
        format!(
            r#"SELECT v_today.vn,
                      os.bw,
                      os.bps,
                      os.bpd,
                      os.pulse
               FROM (
                   SELECT hn, vn
                   FROM   ovst
                   WHERE  vstdate  = '{pd}'
                     AND  cur_dep IN ({ic})
               ) AS v_today
               INNER JOIN (
                   SELECT ov.hn, MAX(ov.vn) AS last_vn
                   FROM   ovst ov
                   INNER JOIN opdscreen osc ON osc.vn = ov.vn
                   WHERE  ov.vstdate = (
                              SELECT MAX(ov3.vstdate)
                              FROM   ovst ov3
                              JOIN   opdscreen os3 ON os3.vn = ov3.vn
                              WHERE  ov3.hn       = ov.hn
                                AND  ov3.vstdate  < '{pd}'
                          )
                   GROUP  BY ov.hn
               ) AS prev ON prev.hn = v_today.hn
               JOIN opdscreen os ON os.vn = prev.last_vn"#,
            pd = process_date,
            ic = in_clause,
        )
    };

    let sql = format!(
        r#"
SELECT
    v.vn,
    v.hn,
    p.cid,
    CONCAT(p.pname, p.fname, ' ', p.lname)            AS pt_name,
    k.department                                       AS current_dept_name,
    pt.name                                            AS pttype_today,

    lv.vstdate                                         AS last_visit_date,

    CAST(IF(vitals.bw  > 0, ROUND(vitals.bw,  0), NULL) AS CHAR) AS last_weight,
    IF(vitals.bps > 0,
        CONCAT(CAST(ROUND(vitals.bps, 0) AS CHAR), '/',
               CAST(ROUND(vitals.bpd, 0) AS CHAR)),
        NULL
    )                                                  AS last_blood_pressure,
    CAST(IF(vitals.pulse > 0, ROUND(vitals.pulse, 0), NULL) AS CHAR) AS last_pulse,

    GROUP_CONCAT(DISTINCT
        IF(h.last_vst IS NOT NULL
           AND DATE_ADD(h.last_vst, INTERVAL m.course_days DAY) <= v.vstdate,
           m.drug_name, NULL)
        ORDER BY m.drug_name SEPARATOR ', '
    )                                                  AS eligible_drugs,

    GROUP_CONCAT(DISTINCT
        IF(h.last_vst IS NULL, m.drug_name, NULL)
        ORDER BY m.drug_name SEPARATOR ', '
    )                                                  AS never_dispensed_drugs,

    GROUP_CONCAT(DISTINCT
        IF(h.last_vst IS NOT NULL
           AND DATE_ADD(h.last_vst, INTERVAL m.course_days DAY) > v.vstdate,
           CONCAT(m.drug_name,
                  ' (in ',
                  DATEDIFF(DATE_ADD(h.last_vst, INTERVAL m.course_days DAY), v.vstdate),
                  ' days|last:',
                  DATE_FORMAT(h.last_vst, '%Y-%m-%d'),
                  ')'),
           NULL)
        ORDER BY m.drug_name SEPARATOR ', '
    )                                                  AS not_yet_eligible_drugs

FROM ovst v
JOIN patient            p   ON p.hn       = v.hn
LEFT JOIN pttype        pt  ON pt.pttype  = v.pttype
LEFT JOIN kskdepartment k   ON k.depcode  = v.cur_dep

/* Last visit date before today — scoped to patients on this day's list */
LEFT JOIN (
    SELECT o2.hn, MAX(o2.vstdate) AS vstdate
    FROM   ovst o2
    WHERE  o2.vstdate < ?
      AND  o2.hn IN (
               SELECT hn FROM ovst
               WHERE  vstdate = ?
                 AND  cur_dep IN ({in_clause})
           )
    GROUP  BY o2.hn
) AS lv ON lv.hn = v.hn

/* Vitals — from the same visit date (vitals_on_date=true) or latest available */
LEFT JOIN (
    {vitals_join}
) AS vitals ON vitals.vn = v.vn

/* Drug master — tiny table, CROSS JOIN is intentional */
CROSS JOIN (
    SELECT icode,
           name AS drug_name,
           CASE icode
{course_days_case}
               ELSE 0
           END AS course_days
    FROM   drugitems
    WHERE  icode IN ({icode_list})
) AS m

/* Dispensing history — scoped to patients on this day to minimise scan */
LEFT JOIN (
    SELECT oi.hn, oi.icode, MAX(o.vstdate) AS last_vst
    FROM   opitemrece oi
    STRAIGHT_JOIN ovst o ON o.vn = oi.vn
                 AND o.vstdate >= DATE_SUB(?, INTERVAL 1 YEAR)
                 AND o.vstdate <  ?
    WHERE  oi.icode IN ({icode_list})
      AND  oi.hn IN (
               SELECT hn FROM ovst
               WHERE  vstdate = ?
                 AND  cur_dep IN ({in_clause})
           )
    GROUP  BY oi.hn, oi.icode
) AS h ON h.hn = v.hn AND h.icode = m.icode

WHERE v.vstdate = ?
  AND v.cur_dep IN ({in_clause})

GROUP BY v.vn, v.hn, p.cid, p.pname, p.fname, p.lname,
         k.department, pt.name, lv.vstdate
ORDER BY v.vn
"#,
        course_days_case = course_days_case,
        icode_list = icode_list,
        in_clause = in_clause,
        vitals_join = vitals_join,
    );

    // Param order matches the ? placeholders left-to-right:
    //  1. lv subquery : o2.vstdate < ?
    //  2. lv subquery : inner SELECT vstdate = ?
    //  3. h subquery  : DATE_SUB(?, INTERVAL 1 YEAR)
    //  4. h subquery  : o.vstdate < ?
    //  5. h subquery  : inner SELECT vstdate = ?
    //  6. WHERE       : v.vstdate = ?
    let params = vec![
        process_date.to_string(), // 1 lv: vstdate < ?
        process_date.to_string(), // 2 lv: inner vstdate = ?
        process_date.to_string(), // 3 h:  DATE_SUB(?, 1 YEAR)
        process_date.to_string(), // 4 h:  o.vstdate < ?
        process_date.to_string(), // 5 h:  inner vstdate = ?
        process_date.to_string(), // 6 WHERE v.vstdate = ?
    ];

    (sql, params)
}

// ---------------------------------------------------------------------------
// Individual search query  (Tab 2)
// ---------------------------------------------------------------------------

pub fn build_individual_search_query(
    process_date: &str,
    hn: Option<&str>,
    cid: Option<&str>,
    name: Option<&str>,
    drugs: &[DrugConfig],
) -> (String, Vec<String>) {
    let icode_list = build_icode_list(drugs);
    let course_days_case = build_course_days_case(drugs);

    let (patient_filter, id_param) = if let Some(h) = hn {
        ("AND p.hn = ?".to_string(), h.to_string())
    } else if let Some(c) = cid {
        ("AND p.cid = ?".to_string(), c.to_string())
    } else if let Some(n) = name {
        (
            "AND CONCAT(p.pname, p.fname, ' ', p.lname) LIKE ?".to_string(),
            format!("%{}%", n),
        )
    } else {
        ("AND 1=0".to_string(), String::new()) // should not happen
    };

    let sql = format!(
        r#"
SELECT
    pat.hn,
    pat.cid,
    pat.pt_name,
    pt_primary.name                            AS pttype_today,
    '' AS vn,
    lv.vstdate                                AS last_visit_date,
    CAST(IF(ls.bw > 0, ROUND(ls.bw, 0), NULL) AS CHAR) AS last_weight,
    IF(ls.bps > 0,
        CONCAT(
            CAST(ROUND(ls.bps, 0) AS CHAR), '/',
            CAST(ROUND(ls.bpd, 0) AS CHAR)
        ),
        NULL
    )                                         AS last_blood_pressure,
    CAST(IF(ls.pulse > 0, ROUND(ls.pulse, 0), NULL) AS CHAR) AS last_pulse,

    GROUP_CONCAT(DISTINCT
        IF(
            h.last_vst IS NOT NULL
            AND DATE_ADD(h.last_vst, INTERVAL m.course_days DAY) <= ?,
            m.drug_name, NULL
        )
        ORDER BY m.drug_name
        SEPARATOR ', '
    )                                         AS eligible_drugs,

    GROUP_CONCAT(DISTINCT
        IF(h.last_vst IS NULL, m.drug_name, NULL)
        ORDER BY m.drug_name
        SEPARATOR ', '
    )                                         AS never_dispensed_drugs,

    GROUP_CONCAT(DISTINCT
        IF(
            h.last_vst IS NOT NULL
            AND DATE_ADD(h.last_vst, INTERVAL m.course_days DAY) > ?,
            CONCAT(
                m.drug_name,
                ' (in ',
                DATEDIFF(
                    DATE_ADD(h.last_vst, INTERVAL m.course_days DAY),
                    ?
                ),
                ' days|last:',
                DATE_FORMAT(h.last_vst, '%Y-%m-%d'),
                ')'
            ),
            NULL
        )
        ORDER BY m.drug_name
        SEPARATOR ', '
    )                                         AS not_yet_eligible_drugs,

    '' AS current_dept_name

FROM (
    SELECT
        p.hn,
        p.cid,
        CONCAT(p.pname, p.fname, ' ', p.lname) AS pt_name,
        p.pttype                              AS pttype
    FROM patient p
    WHERE 1 = 1
      {patient_filter}
    LIMIT 200
) AS pat

LEFT JOIN (
    SELECT o.hn,
           MAX(o.vn) AS last_vn,
           o.vstdate
    FROM   ovst o
    INNER JOIN (
        SELECT hn, MAX(vstdate) AS max_vstdate
        FROM   ovst
        WHERE  vstdate < ?
        GROUP  BY hn
    ) AS mx ON mx.hn = o.hn AND mx.max_vstdate = o.vstdate
    WHERE  o.vstdate < ?
    GROUP  BY o.hn, o.vstdate
) AS lv ON lv.hn = pat.hn

LEFT JOIN pttype    pt_primary ON pt_primary.pttype = pat.pttype
LEFT JOIN opdscreen ls         ON ls.vn             = lv.last_vn

CROSS JOIN (
    SELECT
        icode,
        name AS drug_name,
        CASE icode
{course_days_case}
            ELSE 0
        END AS course_days
    FROM drugitems
    WHERE icode IN ({icode_list})
) AS m

LEFT JOIN (
    SELECT
        oi.hn,
        oi.icode,
        MAX(o.vstdate) AS last_vst
    FROM   opitemrece oi
    STRAIGHT_JOIN ovst o ON o.vn = oi.vn
                        AND o.vstdate >= DATE_SUB(?, INTERVAL 1 YEAR)
                        AND o.vstdate <  ?
    WHERE  oi.icode IN ({icode_list})
    GROUP  BY oi.hn, oi.icode
) AS h ON h.hn = pat.hn AND h.icode = m.icode

GROUP BY pat.hn, pat.cid, pat.pt_name,
         pt_primary.name,
         lv.vstdate, ls.bw, ls.bps, ls.bpd, ls.pulse
ORDER BY pat.hn
"#,
        patient_filter = patient_filter,
        course_days_case = course_days_case,
        icode_list = icode_list,
    );

    let params = vec![
        process_date.to_string(), // eligible: DATE_ADD(...) <= ?
        process_date.to_string(), // not_yet:  DATE_ADD(...) > ?
        process_date.to_string(), // not_yet:  DATEDIFF(..., ?)
        id_param,                 // patient filter
        process_date.to_string(), // lv inner: MAX(vstdate) WHERE vstdate < ?
        process_date.to_string(), // lv outer: WHERE vstdate < ?
        process_date.to_string(), // h:  DATE_SUB(?, INTERVAL 1 YEAR)
        process_date.to_string(), // h:  o.vstdate < ?
    ];

    (sql, params)
}

// ---------------------------------------------------------------------------
// Dispensing history query  (Tab 3)
// ---------------------------------------------------------------------------

pub fn build_dispensing_history_query(
    date_from: &str,
    date_to: &str,
    hn: Option<&str>,
    cid: Option<&str>,
    name: Option<&str>,
    drugs: &[DrugConfig],
) -> (String, Vec<String>) {
    let icode_list = build_icode_list(drugs);

    let (patient_filter, id_param) = if let Some(h) = hn {
        ("AND p.hn = ?".to_string(), Some(h.to_string()))
    } else if let Some(c) = cid {
        ("AND p.cid = ?".to_string(), Some(c.to_string()))
    } else if let Some(n) = name {
        (
            "AND CONCAT(p.pname, p.fname, ' ', p.lname) LIKE ?".to_string(),
            Some(format!("%{}%", n)),
        )
    } else {
        (String::new(), None)
    };

    let sql = format!(
        r#"
SELECT
    o.vstdate,
    p.hn,
    p.cid,
    CONCAT(p.pname, p.fname, ' ', p.lname)  AS pt_name,
    GROUP_CONCAT(
        CONCAT(
            di.name,
            IF(
                oi.qty IS NOT NULL AND oi.qty > 0,
                CONCAT(
                    ' (',
                    CAST(CAST(oi.qty AS SIGNED) AS CHAR),
                    ' ',
                    COALESCE(NULLIF(TRIM(di.units), ''), 'หน่วย'),
                    ')'
                ),
                ''
            )
        )
        ORDER BY di.name
        SEPARATOR ', '
    )                                        AS drugs_dispensed,
    COUNT(DISTINCT oi.icode)                 AS drug_count

FROM opitemrece oi
JOIN ovst       o  ON o.vn    = oi.vn
JOIN patient    p  ON p.hn    = oi.hn
JOIN drugitems  di ON di.icode = oi.icode

WHERE oi.icode IN ({icode_list})
  {patient_filter}
  AND o.vstdate BETWEEN ? AND ?

GROUP BY o.vstdate, p.hn, p.cid, p.pname, p.fname, p.lname
ORDER BY o.vstdate DESC, p.hn
LIMIT 1000
"#,
        icode_list = icode_list,
        patient_filter = patient_filter,
    );

    let mut params = Vec::new();
    if let Some(id) = id_param {
        params.push(id);
    }
    params.push(date_from.to_string()); // BETWEEN ?
    params.push(date_to.to_string()); // AND ?

    (sql, params)
}

// ---------------------------------------------------------------------------
// Patient herb history query  (per-patient tree view)
// ---------------------------------------------------------------------------

pub fn build_patient_herb_history_query(
    hn: &str,
    years_back: Option<i32>,
    drugs: &[DrugConfig],
) -> (String, Vec<String>) {
    let icode_list = build_icode_list(drugs);

    let date_filter = match years_back {
        Some(y) if y > 0 => format!("AND o.vstdate >= DATE_SUB(CURDATE(), INTERVAL {} YEAR)", y),
        _ => String::new(),
    };

    let sql = format!(
        r#"
SELECT
    o.vstdate,
    di.name                                       AS drug_name,
    CAST(CAST(oi.qty AS SIGNED) AS CHAR)          AS qty,
    COALESCE(NULLIF(TRIM(di.units), ''), 'หน่วย') AS units

FROM opitemrece oi
JOIN ovst      o  ON o.vn     = oi.vn
JOIN drugitems di ON di.icode = oi.icode

WHERE oi.hn = ?
  AND oi.icode IN ({icode_list})
  {date_filter}

ORDER BY o.vstdate DESC, di.name
LIMIT 5000
"#,
        icode_list = icode_list,
        date_filter = date_filter,
    );

    (sql, vec![hn.to_string()])
}

// ---------------------------------------------------------------------------
// Patient name lookup
// ---------------------------------------------------------------------------

pub fn build_patient_lookup_by_name(name: &str) -> (String, Vec<String>) {
    let sql = r#"
SELECT
    p.hn,
    p.cid,
    CONCAT(p.pname, p.fname, ' ', p.lname) AS pt_name,
    COALESCE(pt.name, '') AS pttype_name
FROM patient p
LEFT JOIN pttype pt ON pt.pttype = p.pttype
WHERE CONCAT(p.pname, p.fname, ' ', p.lname) LIKE ?
ORDER BY p.hn
LIMIT 200
"#
    .to_string();

    (sql, vec![format!("%{}%", name)])
}

// ---------------------------------------------------------------------------
// Patient lookup by HN or CID
// ---------------------------------------------------------------------------

/// Build a query to look up a single patient by HN (if is_cid=false) or CID (if is_cid=true).
/// Returns hn, cid, pt_name, pttype_name.
pub fn build_patient_lookup_by_hn_or_cid(id: &str, is_cid: bool) -> (String, Vec<String>) {
    let where_clause = if is_cid {
        "WHERE p.cid = ?"
    } else {
        "WHERE p.hn = ?"
    };

    let sql = format!(
        r#"
SELECT
    p.hn,
    p.cid,
    CONCAT(p.pname, p.fname, ' ', p.lname) AS pt_name,
    COALESCE(pt.name, '') AS pttype_name
FROM patient p
LEFT JOIN pttype pt ON pt.pttype = p.pttype
{where_clause}
LIMIT 1
"#,
        where_clause = where_clause,
    );

    (sql, vec![id.to_string()])
}
