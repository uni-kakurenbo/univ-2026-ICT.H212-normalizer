use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) const MAX_STATES: usize = 128;
pub(super) const MAX_EDGES: usize = 4096;
const STEPS: usize = 200_000;
const ALPHABET: [u8; 2] = *b"ab";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Machine {
    pub states: BTreeSet<String>,
    pub start: String,
    pub finals: BTreeSet<String>,
    pub edges: BTreeSet<(String, u8, String)>,
}

impl Machine {
    pub fn new(start: &str) -> Self {
        Self {
            states: BTreeSet::from([start.to_owned()]),
            start: start.to_owned(),
            finals: BTreeSet::new(),
            edges: BTreeSet::new(),
        }
    }
    pub fn edge(&mut self, from: &str, symbol: u8, to: &str) -> Result<()> {
        self.states.insert(from.to_owned());
        self.states.insert(to.to_owned());
        self.edges.insert((from.to_owned(), symbol, to.to_owned()));
        self.bound()
    }
    pub fn bound(&self) -> Result<()> {
        if self.states.len() > MAX_STATES || self.edges.len() > MAX_EDGES {
            Err(Unavailable::Limit)
        } else {
            Ok(())
        }
    }
    pub fn same_relation(&self, other: &Self) -> bool {
        // Starts are implicit in ex-1/2/3; ex-4 has no start-state field.
        self.states == other.states && self.finals == other.finals && self.edges == other.edges
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unavailable {
    Syntax,
    Limit,
    NeedsDfa,
    Epsilon,
    EmptySubset,
    LabelCollision,
    NoAcceptingState,
}
pub(super) type Result<T> = std::result::Result<T, Unavailable>;

struct Budget(usize);
impl Budget {
    fn new() -> Self {
        Self(STEPS)
    }
    fn step(&mut self) -> Result<()> {
        self.0 = self.0.checked_sub(1).ok_or(Unavailable::Limit)?;
        Ok(())
    }
}

struct Indexed {
    names: Vec<String>,
    start: usize,
    finals: Vec<bool>,
    // a, b, epsilon. Build once to avoid scanning every edge at every step.
    edges: Vec<[Vec<usize>; 3]>,
}
impl Indexed {
    fn from(m: &Machine) -> Result<Self> {
        m.bound()?;
        let names: Vec<_> = m.states.iter().cloned().collect();
        let ids: BTreeMap<_, _> = names
            .iter()
            .enumerate()
            .map(|(i, s)| (s.as_str(), i))
            .collect();
        let start = *ids.get(m.start.as_str()).ok_or(Unavailable::Syntax)?;
        let finals = names.iter().map(|s| m.finals.contains(s)).collect();
        let mut edges = vec![[Vec::new(), Vec::new(), Vec::new()]; names.len()];
        for (from, symbol, to) in &m.edges {
            let column = match symbol {
                b'a' => 0,
                b'b' => 1,
                b'$' => 2,
                _ => return Err(Unavailable::Syntax),
            };
            let from = *ids.get(from.as_str()).ok_or(Unavailable::Syntax)?;
            let to = *ids.get(to.as_str()).ok_or(Unavailable::Syntax)?;
            edges[from][column].push(to);
        }
        Ok(Self {
            names,
            start,
            finals,
            edges,
        })
    }
    fn no_epsilon(&self) -> Result<()> {
        if self.edges.iter().any(|row| !row[2].is_empty()) {
            Err(Unavailable::Epsilon)
        } else {
            Ok(())
        }
    }
    fn dfa(&self, total: bool) -> Result<()> {
        self.no_epsilon()?;
        if self.edges.iter().any(|row| {
            row[..2]
                .iter()
                .any(|set| set.len() > 1 || (total && set.len() != 1))
        }) {
            Err(Unavailable::NeedsDfa)
        } else {
            Ok(())
        }
    }
    fn closures(&self, budget: &mut Budget) -> Result<Vec<BTreeSet<usize>>> {
        let mut all = Vec::new();
        for root in 0..self.names.len() {
            let mut seen = BTreeSet::from([root]);
            let mut pending = vec![root];
            while let Some(q) = pending.pop() {
                budget.step()?;
                for &to in &self.edges[q][2] {
                    budget.step()?;
                    if seen.insert(to) {
                        pending.push(to);
                    }
                }
            }
            all.push(seen);
        }
        Ok(all)
    }
}

pub(super) fn complement(m: &Machine, sink: &str) -> Result<Machine> {
    let index = Indexed::from(m)?;
    index.dfa(false)?;
    if m.states.contains(sink) {
        return Err(Unavailable::LabelCollision);
    }
    let mut result = m.clone();
    let mut needed = false;
    for (q, name) in index.names.iter().enumerate() {
        for (column, symbol) in ALPHABET.iter().enumerate() {
            if index.edges[q][column].is_empty() {
                result.edge(name, *symbol, sink)?;
                needed = true;
            }
        }
    }
    if needed {
        for symbol in ALPHABET {
            result.edge(sink, symbol, sink)?;
        }
    }
    result.finals = result.states.difference(&m.finals).cloned().collect();
    result.bound()?;
    Ok(result)
}

pub(super) fn union(first: &Machine, second: &Machine, start: &str) -> Result<Machine> {
    if !first.states.is_disjoint(&second.states)
        || first.states.contains(start)
        || second.states.contains(start)
    {
        return Err(Unavailable::LabelCollision);
    }
    let mut result = Machine::new(start);
    result
        .states
        .extend(first.states.iter().chain(&second.states).cloned());
    result
        .finals
        .extend(first.finals.iter().chain(&second.finals).cloned());
    result
        .edges
        .extend(first.edges.iter().chain(&second.edges).cloned());
    result.edge(start, b'$', &first.start)?;
    result.edge(start, b'$', &second.start)?;
    result.bound()?;
    Ok(result)
}

// Lecture 05 p.6: close BEFORE and AFTER the consumed symbol. F_n retains
// F_epsilon and adds only the start state when it epsilon-reaches a final.
// Preserve all original states; no unreachable/dead-state cleanup here.
pub(super) fn remove_epsilon(m: &Machine) -> Result<Machine> {
    let index = Indexed::from(m)?;
    let mut budget = Budget::new();
    let closures = index.closures(&mut budget)?;
    let mut result = Machine::new(&m.start);
    result.states.clone_from(&m.states);
    result.finals.clone_from(&m.finals);
    if closures[index.start].iter().any(|&q| index.finals[q]) {
        result.finals.insert(m.start.clone());
    }
    for (q, closure) in closures.iter().enumerate() {
        for &before in closure {
            for (column, symbol) in ALPHABET.iter().enumerate() {
                budget.step()?;
                for &after in &index.edges[before][column] {
                    for &to in &closures[after] {
                        budget.step()?;
                        result.edge(&index.names[q], *symbol, &index.names[to])?;
                    }
                }
            }
        }
    }
    Ok(result)
}

// Reachable subset construction; never guess an unspecified spelling of {}.
pub(super) fn determinize(m: &Machine) -> Result<Machine> {
    let index = Indexed::from(m)?;
    index.no_epsilon()?;
    let mut budget = Budget::new();
    let initial = BTreeSet::from([index.start]);
    let mut subsets = BTreeMap::from([(initial.clone(), m.start.clone())]);
    let mut labels = BTreeSet::from([m.start.clone()]);
    let mut pending = VecDeque::from([initial]);
    let mut result = Machine::new(&m.start);
    while let Some(set) = pending.pop_front() {
        let label = subsets[&set].clone();
        if set.iter().any(|&q| index.finals[q]) {
            result.finals.insert(label.clone());
        }
        for (column, symbol) in ALPHABET.iter().enumerate() {
            let mut next = BTreeSet::new();
            for &q in &set {
                budget.step()?;
                for &to in &index.edges[q][column] {
                    budget.step()?;
                    next.insert(to);
                }
            }
            if next.is_empty() {
                return Err(Unavailable::EmptySubset);
            }
            if !subsets.contains_key(&next) {
                if subsets.len() >= MAX_STATES {
                    return Err(Unavailable::Limit);
                }
                let name: String = next.iter().map(|&q| index.names[q].as_str()).collect();
                if !labels.insert(name.clone()) {
                    return Err(Unavailable::LabelCollision);
                }
                subsets.insert(next.clone(), name);
                pending.push_back(next.clone());
            }
            result.edge(&label, *symbol, &subsets[&next])?;
        }
    }
    Ok(result)
}

// Lecture 06 p.6: refine the final/nonfinal partition until stable, form the
// quotient, and remove unreachable/non-coaccessible states. This is scoped to
// the ex-3 -> ex-4 procedure, never a general equivalence/minimality service.
pub(super) fn minimize(m: &Machine) -> Result<Machine> {
    let index = Indexed::from(m)?;
    index.dfa(true)?;
    let mut budget = Budget::new();
    let mut reachable = BTreeSet::from([index.start]);
    let mut pending = vec![index.start];
    while let Some(q) = pending.pop() {
        for column in 0..2 {
            budget.step()?;
            let to = index.edges[q][column][0];
            if reachable.insert(to) {
                pending.push(to);
            }
        }
    }
    let mut classes: Vec<usize> = index.finals.iter().map(|&f| usize::from(f)).collect();
    loop {
        let mut signatures = BTreeMap::new();
        let mut next = classes.clone();
        for &q in &reachable {
            budget.step()?;
            let signature = [
                usize::from(index.finals[q]),
                classes[index.edges[q][0][0]],
                classes[index.edges[q][1][0]],
            ];
            let fresh = signatures.len();
            next[q] = *signatures.entry(signature).or_insert(fresh);
        }
        // Canonical IDs can initially swap; equality of the partition rather
        // than numeric ID equality avoids falsely stopping or oscillating.
        let mut old_to_new = BTreeMap::new();
        let mut new_to_old = BTreeMap::new();
        let stable = reachable.iter().all(|&q| {
            let a = old_to_new.entry(classes[q]).or_insert(next[q]);
            let b = new_to_old.entry(next[q]).or_insert(classes[q]);
            *a == next[q] && *b == classes[q]
        });
        classes = next;
        if stable {
            break;
        }
    }
    let mut reverse = vec![Vec::new(); index.names.len()];
    for &q in &reachable {
        for column in 0..2 {
            reverse[index.edges[q][column][0]].push(q);
        }
    }
    let mut live: BTreeSet<_> = reachable
        .iter()
        .copied()
        .filter(|&q| index.finals[q])
        .collect();
    let mut pending: Vec<_> = live.iter().copied().collect();
    while let Some(q) = pending.pop() {
        for &before in &reverse[q] {
            budget.step()?;
            if live.insert(before) {
                pending.push(before);
            }
        }
    }
    if !live.contains(&index.start) {
        return Err(Unavailable::NoAcceptingState);
    }
    let mut groups: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for &q in &live {
        groups.entry(classes[q]).or_default().push(&index.names[q]);
    }
    let mut sorted: Vec<_> = groups
        .into_iter()
        .map(|(id, names)| (format!("{{{}}}", names.join(",")), id))
        .collect();
    sorted.sort_unstable(); // WHOLE rendered set, including ',' and '}'.
    let labels: BTreeMap<_, _> = sorted
        .iter()
        .enumerate()
        .map(|(i, (_, id))| (*id, format!("p{}", i + 1)))
        .collect();
    let mut result = Machine::new(&labels[&classes[index.start]]);
    result.states.extend(labels.values().cloned());
    for &q in &live {
        let from = &labels[&classes[q]];
        if index.finals[q] {
            result.finals.insert(from.clone());
        }
        for (column, symbol) in ALPHABET.iter().enumerate() {
            budget.step()?;
            let to = index.edges[q][column][0];
            if live.contains(&to) {
                result.edge(from, *symbol, &labels[&classes[to]])?;
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn machine(start: &str, finals: &[&str], edges: &[(&str, u8, &str)]) -> Machine {
        let mut m = Machine::new(start);
        for &s in finals {
            m.states.insert(s.to_owned());
            m.finals.insert(s.to_owned());
        }
        for &(from, symbol, to) in edges {
            m.edge(from, symbol, to).unwrap();
        }
        m
    }
    fn has(m: &Machine, from: &str, symbol: u8, to: &str) -> bool {
        m.edges.contains(&(from.to_owned(), symbol, to.to_owned()))
    }

    #[test]
    fn complement_completes_before_inverting_and_union_keeps_both_branches() {
        let left = machine("X", &["Y"], &[("X", b'a', "Y")]);
        let right = machine("I", &["F"], &[("I", b'b', "F")]);
        let complement = complement(&right, "D").unwrap();
        assert_eq!(
            complement.finals,
            BTreeSet::from(["D".to_owned(), "I".to_owned()])
        );
        assert!(has(&complement, "I", b'a', "D"));
        assert!(has(&complement, "I", b'b', "F"));
        for q in ["D", "F"] {
            for a in ALPHABET {
                assert!(has(&complement, q, a, "D"));
            }
        }
        let combined = union(&left, &complement, "O").unwrap();
        assert!(has(&combined, "O", b'$', "X") && has(&combined, "O", b'$', "I"));
        assert!(combined.finals.contains("Y") && !combined.finals.contains("F"));
        assert_eq!(union(&left, &left, "O"), Err(Unavailable::LabelCollision));
    }

    #[test]
    fn epsilon_removal_closes_both_sides_handles_cycles_and_adds_only_start_to_finals() {
        let m = machine(
            "I",
            &["F"],
            &[
                ("I", b'$', "X"),
                ("I", b'$', "F"),
                ("X", b'$', "Y"),
                ("Y", b'$', "X"),
                ("Y", b'a', "T"),
                ("T", b'$', "F"),
                ("U", b'$', "F"),
                ("V", b'b', "V"),
            ],
        );
        let result = remove_epsilon(&m).unwrap();
        assert_eq!(result.states, m.states); // includes unreachable U/V
        assert_eq!(
            result.finals,
            BTreeSet::from(["F".to_owned(), "I".to_owned()])
        );
        assert!(!result.finals.contains("U"));
        for q in ["I", "X", "Y"] {
            for target in ["T", "F"] {
                assert!(has(&result, q, b'a', target));
            }
        }
        assert!(has(&result, "V", b'b', "V"));
        assert!(result.edges.iter().all(|t| t.1 != b'$'));
    }

    #[test]
    fn subset_construction_visits_only_reachable_sets_with_sorted_members_and_any_final() {
        let m = machine(
            "X",
            &["Y"],
            &[
                ("X", b'a', "X"),
                ("X", b'a', "Y"),
                ("X", b'b', "X"),
                ("Y", b'a', "Y"),
                ("Y", b'b', "Y"),
                ("U", b'a', "U"),
                ("U", b'b', "U"),
            ],
        );
        let result = determinize(&m).unwrap();
        assert_eq!(
            result.states,
            BTreeSet::from(["X".to_owned(), "XY".to_owned()])
        );
        assert_eq!(result.finals, BTreeSet::from(["XY".to_owned()]));
        assert_eq!(result.start, "X");
        assert!(has(&result, "X", b'a', "XY") && has(&result, "X", b'b', "X"));
        assert!(has(&result, "XY", b'a', "XY") && has(&result, "XY", b'b', "XY"));
    }

    #[test]
    fn minimization_merges_equivalent_states_and_numbers_whole_braced_class_strings() {
        let m = machine(
            "X",
            &["XY", "XZ", "U"],
            &[
                ("X", b'a', "XY"),
                ("X", b'b', "X"),
                ("XY", b'a', "XZ"),
                ("XY", b'b', "XZ"),
                ("XZ", b'a', "XY"),
                ("XZ", b'b', "XY"),
                ("U", b'a', "U"),
                ("U", b'b', "U"),
            ],
        );
        let result = minimize(&m).unwrap();
        // "{XY,XZ}" < "{X}" in ASCII although ["X"] < ["XY","XZ"].
        assert_eq!(result.start, "p2");
        assert_eq!(result.states.len(), 2);
        assert_eq!(result.finals, BTreeSet::from(["p1".to_owned()]));
        assert!(has(&result, "p2", b'a', "p1") && has(&result, "p2", b'b', "p2"));
        assert!(has(&result, "p1", b'a', "p1") && has(&result, "p1", b'b', "p1"));
    }

    #[test]
    fn minimization_refines_beyond_one_step_and_follows_lecture_dead_state_cleanup() {
        let mut m = Machine::new("X0");
        m.finals.insert("X8".to_owned());
        for i in 0..8 {
            for symbol in ALPHABET {
                m.edge(&format!("X{i}"), symbol, &format!("X{}", i + 1))
                    .unwrap();
            }
        }
        for symbol in ALPHABET {
            m.edge("X8", symbol, "D").unwrap();
            m.edge("D", symbol, "D").unwrap();
        }
        let result = minimize(&m).unwrap();
        assert_eq!(result.states.len(), 9); // each remaining distance to F differs
        assert_eq!(result.finals.len(), 1);
        assert_eq!(result.edges.len(), 16); // dead quotient is removed (lecture 06)
    }

    #[test]
    fn missing_or_nondeterministic_dependencies_and_unspecified_empty_sets_are_unavailable() {
        let mut m = machine("X", &["Y"], &[("X", b'a', "Y")]);
        assert_eq!(determinize(&m), Err(Unavailable::EmptySubset));
        assert_eq!(minimize(&m), Err(Unavailable::NeedsDfa));
        m.edge("X", b'a', "X").unwrap();
        assert_eq!(minimize(&m), Err(Unavailable::NeedsDfa));
        m.edge("X", b'$', "Y").unwrap();
        assert_eq!(determinize(&m), Err(Unavailable::Epsilon));
    }

    #[test]
    fn dense_epsilon_graphs_exhaust_budget_and_state_and_edge_caps_are_enforced() {
        let mut m = Machine::new("X0");
        for i in 0..64 {
            for j in 0..64 {
                m.edge(&format!("X{i}"), b'$', &format!("X{j}")).unwrap();
            }
        }
        assert_eq!(remove_epsilon(&m), Err(Unavailable::Limit));
        assert_eq!(m.edge("X0", b'a', "X0"), Err(Unavailable::Limit));
        let mut m = Machine::new("X0");
        for i in 1..MAX_STATES {
            m.edge("X0", b'a', &format!("X{i}")).unwrap();
        }
        assert_eq!(m.edge("X0", b'a', "overflow"), Err(Unavailable::Limit));
    }
}
