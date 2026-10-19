use super::expressions::{self, Expr, Form, given, limit};
use super::{AnswerLine, edge_trim, triple};
use crate::{Report, Severity};
use std::collections::{BTreeMap, BTreeSet};

fn issue(r: &mut Report, code: &'static str, line: usize, message: &str) {
    r.issue(Severity::Error, code, Some(line), message);
}

fn set(r: &mut Report, value: &str, line: usize) -> Option<BTreeSet<Expr>> {
    let inner = value.strip_prefix('{')?.strip_suffix('}')?;
    let mut result = BTreeSet::new();
    if !inner.is_empty() {
        for (i, e) in inner.split(',').enumerate() {
            if i >= 512 {
                limit(r, Some(line));
                return None;
            }
            result.insert(expressions::read(r, e, line)?);
        }
    }
    Some(result)
}

fn form_row(r: &mut Report, value: &str, line: usize) -> Option<(Expr, Form)> {
    let (p, value) = value.strip_prefix("lf(")?.split_once(")=")?;
    let p = expressions::read(r, p, line)?;
    let value = value.strip_prefix('{')?.strip_suffix('}')?;
    let mut result = Form::new();
    if !value.is_empty() {
        for (i, pair) in value
            .strip_prefix('<')?
            .strip_suffix('>')?
            .split(">,<")
            .enumerate()
        {
            if i >= 512 {
                limit(r, Some(line));
                return None;
            }
            let (c, q) = pair.split_once(',')?;
            if !matches!(c, "a" | "b" | "c") {
                issue(
                    r,
                    "mechanical-alphabet",
                    line,
                    "linear form の先頭記号は a・b・c のいずれか1記号です。εや定数は先頭記号になりません。",
                );
                return None;
            }
            result.insert((c.as_bytes()[0], expressions::read(r, q, line)?));
        }
    }
    Some((p, result))
}

struct Stage {
    known: BTreeSet<Expr>,
    delta: BTreeSet<Expr>,
    line: usize,
}

// Slide 9's worked example resolves the nullable-concatenation branch: include
// lf(beta) precisely when alpha accepts epsilon. No language-equivalence oracle
// or stored target answers are used; comparison follows the symbolic rules.
pub(super) fn check_steps(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_steps_for(r, rows, given().normal());
}

fn check_steps_for(r: &mut Report, rows: &[&AnswerLine<'_>], initial: Expr) {
    if rows.len() > 2048 {
        limit(r, None);
        return;
    }
    let mut budget = 200_000;
    let mut forms = BTreeMap::new();
    let mut stages = Vec::new();
    for row in rows {
        let value = edge_trim(row.text);
        if value.starts_with("lf(") {
            let Some((p, supplied)) = form_row(r, value, row.number) else {
                return;
            };
            let Ok(expected) = p.form(&mut budget) else {
                limit(r, Some(row.number));
                return;
            };
            if supplied != expected {
                issue(
                    r,
                    "mechanical-linear-form",
                    row.number,
                    "lf(p) の内容が講義の linear form の再帰規則に一致しません。和・積・* と空列の扱いを確認してください。",
                );
            }
            forms.insert(p, expected);
        } else {
            let Some((_, tail)) = value.strip_prefix("i=").and_then(|v| v.split_once(",K=")) else {
                return;
            };
            let Some((known, delta)) = tail.split_once(",Delta=") else {
                return;
            };
            let Some(known) = set(r, known, row.number) else {
                return;
            };
            let Some(delta) = set(r, delta, row.number) else {
                return;
            };
            stages.push(Stage {
                known,
                delta,
                line: row.number,
            });
        }
    }
    let Some(first) = stages.first() else {
        return;
    };
    if !first.known.is_empty() || first.delta != BTreeSet::from([initial]) {
        issue(
            r,
            "mechanical-stage-initial",
            first.line,
            "初期値は K_0={}、Delta_0={e} としてください。",
        );
    }
    for (i, stage) in stages.iter().enumerate() {
        if i + 1 < stages.len() && stage.delta.is_empty() {
            issue(
                r,
                "mechanical-stage-after-end",
                stages[i + 1].line,
                "Delta が空になった段階で反復は終了します。終了後の段階が記載されています。",
            );
        }
        if let Some(next) = stages.get(i + 1) {
            let known: BTreeSet<_> = stage.known.union(&stage.delta).cloned().collect();
            let mut delta = BTreeSet::new();
            for p in &stage.delta {
                if !forms.contains_key(p) {
                    issue(
                        r,
                        "mechanical-linear-form-missing",
                        stage.line,
                        "処理する Delta 内の正規表現に対する lf(p) の行がありません。",
                    );
                }
                let Ok(form) = p.form(&mut budget) else {
                    limit(r, Some(stage.line));
                    return;
                };
                delta.extend(
                    form.into_iter()
                        .map(|(_, q)| q)
                        .filter(|q| !known.contains(q)),
                );
            }
            if next.known != known {
                issue(
                    r,
                    "mechanical-stage-known",
                    next.line,
                    "K の更新が K_{i+1}=K_i∪Delta_i に一致しません。",
                );
            }
            if next.delta != delta {
                issue(
                    r,
                    "mechanical-stage-delta",
                    next.line,
                    "Delta の更新が講義の規則に一致しません。lf(p) の到着先から、更新後の K にある状態を除いてください。",
                );
            }
        }
    }
    if let Some(last) = stages.last()
        && !last.delta.is_empty()
    {
        issue(
            r,
            "mechanical-stage-unfinished",
            last.line,
            "反復は Delta が空になる段階まで記載してください。",
        );
    }
}

pub(super) fn check_automaton(r: &mut Report, rows: &[&AnswerLine<'_>]) {
    check_automaton_for(r, rows, true);
}

fn check_automaton_for(r: &mut Report, rows: &[&AnswerLine<'_>], require_e: bool) {
    if rows.len() > 2048 {
        limit(r, None);
        return;
    }
    let Some(start) = expressions::read(r, edge_trim(rows[0].text), rows[0].number) else {
        return;
    };
    if require_e && edge_trim(rows[0].text) != "e" {
        issue(
            r,
            "mechanical-top-down-start",
            rows[0].number,
            "トップダウン法の出発状態は課題の正規表現全体の別名 e です。",
        );
    }
    let mut finals = BTreeSet::new();
    let mut states = BTreeMap::from([(start.clone(), rows[0].number)]);
    for text in edge_trim(rows[1].text).split(',') {
        let Some(q) = expressions::read(r, text, rows[1].number) else {
            return;
        };
        states.insert(q.clone(), rows[1].number);
        finals.insert(q);
    }
    let mut transitions: BTreeMap<Expr, Form> = BTreeMap::new();
    for row in &rows[2..] {
        let Some([p, c, q]) = triple(edge_trim(row.text)) else {
            return;
        };
        let Some(p) = expressions::read(r, p, row.number) else {
            return;
        };
        let Some(q) = expressions::read(r, q, row.number) else {
            return;
        };
        states.entry(p.clone()).or_insert(row.number);
        states.entry(q.clone()).or_insert(row.number);
        transitions
            .entry(p)
            .or_default()
            .insert((c.as_bytes()[0], q));
    }
    let mut budget = 200_000;
    for (p, line) in &states {
        let Ok(expected) = p.form(&mut budget) else {
            limit(r, Some(*line));
            return;
        };
        let empty = Form::new();
        if transitions.get(p).unwrap_or(&empty) != &expected {
            issue(
                r,
                "mechanical-top-down-transitions",
                *line,
                "この状態の遷移が lf(p) から得られる遷移集合に一致しません。欠落・余分な遷移や到着先を確認してください。",
            );
        }
    }
    let expected_finals: BTreeSet<_> = states.keys().filter(|p| p.nullable()).cloned().collect();
    if finals != expected_finals {
        issue(
            r,
            "mechanical-top-down-finals",
            rows[1].number,
            "最終状態の集合が、空列を含む状態の集合 F={p∈K | ε∈L(p)} に一致しません。",
        );
    }
    let mut reachable = BTreeSet::from([start.clone()]);
    let mut pending = vec![start];
    while let Some(p) = pending.pop() {
        if let Some(edges) = transitions.get(&p) {
            for (_, q) in edges {
                if reachable.insert(q.clone()) {
                    pending.push(q.clone());
                }
            }
        }
    }
    if states.keys().any(|p| !reachable.contains(p)) {
        issue(
            r,
            "mechanical-top-down-states",
            rows[0].number,
            "出発状態から追加される状態集合に含まれない状態があります。トップダウン法の状態追加規則を確認してください。",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(s: &str) -> Expr {
        Expr::parse(s).unwrap().normal()
    }
    #[test]
    fn recursive_linear_form_rules_cover_constants_union_nullable_product_and_star() {
        let mut budget = 10000;
        assert_eq!(e("0").form(&mut budget).unwrap(), Form::new());
        assert_eq!(e("1").form(&mut budget).unwrap(), Form::new());
        assert_eq!(
            e("a").form(&mut budget).unwrap(),
            Form::from([(b'a', e("1"))])
        );
        assert_eq!(
            e("a+b+c").form(&mut budget).unwrap(),
            Form::from([(b'a', e("1")), (b'b', e("1")), (b'c', e("1"))])
        );
        assert_eq!(
            e("a*b").form(&mut budget).unwrap(),
            Form::from([(b'a', e("a*b")), (b'b', e("1"))])
        );
        assert_eq!(
            e("ab").form(&mut budget).unwrap(),
            Form::from([(b'a', e("b"))])
        );
        assert_eq!(e("a0").form(&mut budget).unwrap(), Form::new());
        assert!(e("(a+1)*").nullable());
        assert!(!e("(a+1)*b").nullable());
    }

    fn steps(text: &str) -> Report {
        let rows: Vec<_> = text
            .lines()
            .enumerate()
            .map(|(i, text)| AnswerLine {
                text,
                number: i + 1,
            })
            .collect();
        let mut r = Report::default();
        check_steps_for(&mut r, &rows.iter().collect::<Vec<_>>(), e("a"));
        r
    }
    fn automaton(text: &str) -> Report {
        let rows: Vec<_> = text
            .lines()
            .enumerate()
            .map(|(i, text)| AnswerLine {
                text,
                number: i + 1,
            })
            .collect();
        let mut r = Report::default();
        check_automaton_for(&mut r, &rows.iter().collect::<Vec<_>>(), false);
        r
    }
    #[test]
    fn linear_forms_and_iteration_values_are_checked_with_a_small_unrelated_expression() {
        let valid = "lf(a)={<a,1>}\nlf(1)={}\ni=0,K={},Delta={a}\ni=1,K={a},Delta={1}\ni=2,K={1,a},Delta={}";
        assert_eq!(steps(valid).errors(), 0);
        assert_eq!(
            steps(
                &valid
                    .replace("<a,1>", "<a,(1)>")
                    .replace("K={1,a}", "K={a,1}")
            )
            .errors(),
            0
        );
        for (before, after, code) in [
            ("<a,1>", "<b,1>", "mechanical-linear-form"),
            ("i=0,K={}", "i=0,K={a}", "mechanical-stage-initial"),
            ("i=1,K={a}", "i=1,K={}", "mechanical-stage-known"),
            (
                "i=1,K={a},Delta={1}",
                "i=1,K={a},Delta={b}",
                "mechanical-stage-delta",
            ),
            ("lf(1)={}\n", "", "mechanical-linear-form-missing"),
            ("\ni=2,K={1,a},Delta={}", "", "mechanical-stage-unfinished"),
        ] {
            let r = steps(&valid.replace(before, after));
            assert!(
                r.issues.iter().any(|i| i.code == code),
                "{code}: {:?}",
                r.issues
            );
        }
        assert!(
            steps(&format!("{valid}\ni=3,K={{1,a}},Delta={{}}"))
                .issues
                .iter()
                .any(|i| i.code == "mechanical-stage-after-end")
        );
    }
    #[test]
    fn top_down_graph_matches_linear_forms_nullable_finals_and_reachable_states() {
        assert_eq!(automaton("a\n1\n(a,a,1)").errors(), 0);
        for (text, code) in [
            ("a\n1\n(a,b,1)", "mechanical-top-down-transitions"),
            ("a\na\n(a,a,1)", "mechanical-top-down-finals"),
            ("a\n1\n(a,a,1)\n(b,b,1)", "mechanical-top-down-states"),
            ("a\n1\n(a,$,1)", "mechanical-top-down-transitions"),
        ] {
            let r = automaton(text);
            assert!(
                r.issues.iter().any(|i| i.code == code),
                "{code}: {:?}",
                r.issues
            );
        }
    }
}
