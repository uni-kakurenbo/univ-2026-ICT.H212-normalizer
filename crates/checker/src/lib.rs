pub mod assignments;

use sha2::{Digest, Sha256};

pub const HASH_VERSION: &str = "answer-text-v1";
pub const MAX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Compact,
    Prose,
    Productions,
    DfaTransitions,
    PngFilename,
}

pub struct Section {
    pub id: &'static str,
    pub marker: &'static str,
    pub ascii: bool,
    pub format: Format,
    pub validate: Option<Validator>,
}

pub type Validator = fn(&mut Report, &[&AnswerLine<'_>]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub code: &'static str,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Clone, Default, Debug)]
pub struct FileInfo {
    pub bytes: usize,
    pub bom: bool,
    pub lf: usize,
    pub crlf: usize,
    pub cr: usize,
}

#[derive(Clone, Default, Debug)]
pub struct Report {
    pub issues: Vec<Issue>,
    pub info: FileInfo,
    pub normalized: Option<String>,
    pub hash_input: Option<String>,
    pub hash: Option<String>,
    pub changes: Vec<&'static str>,
}

impl Report {
    pub fn issue(
        &mut self,
        severity: Severity,
        code: &'static str,
        line: Option<usize>,
        message: impl Into<String>,
    ) {
        self.issues.push(Issue {
            severity,
            code,
            line,
            message: message.into(),
        });
    }
    pub fn errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }
    pub fn warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
}

#[derive(Clone)]
pub struct AnswerLine<'a> {
    pub text: &'a str,
    pub number: usize,
}

fn is_comment(text: &str) -> bool {
    text.trim_start().starts_with('#')
}

/// Assignment 04 uses '#' and '#AB' as states. Before any answer in a
/// transition section, '#' followed by an uppercase letter, comma or end
/// starts a final-state list. Keep even malformed candidates for diagnostics.
/// '###' instructions and '# comment' remain comments. An ambiguous '#A...'
/// comment in that position must instead use '###'. Never reinterpret later
/// comment rows or change the behavior of assignments 01–03.
fn hash_final_state_candidate(text: &str) -> bool {
    text.trim().strip_prefix('#').is_some_and(|tail| {
        tail.is_empty() || tail.starts_with(|c: char| c.is_ascii_uppercase() || c == ',')
    })
}
pub(crate) fn edge_trim(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

fn looks_like_marker(text: &str) -> bool {
    let Some((id, equals)) = text.split_once('=') else {
        return false;
    };
    !id.is_empty()
        && id.starts_with(|c: char| c.is_ascii_digit())
        && id
            .chars()
            .all(|c| c.is_ascii_digit() || c == '-' || c == '.')
        && equals.chars().all(|c| c == '=')
}

/// Inspect bytes, never replace invalid UTF-8 with replacement characters.
/// Hash v1 is UTF-8 without BOM, LF, exact template markers and one blank line
/// between sections, exactly one final LF. Only recognized answer sections
/// enter the hash. Header values and whole-line # comments never enter it.
/// A '#' state in an opted-in final-state list is answer text, not a comment.
pub fn check(assignment_id: &str, bytes: &[u8]) -> Report {
    use Severity::*;
    let mut r = Report::default();
    r.info.bytes = bytes.len();
    let Some(assignment) = assignments::get(assignment_id) else {
        r.issue(
            Error,
            "assignment",
            None,
            "対応する課題を選択してください。",
        );
        return r;
    };
    if bytes.len() > MAX_BYTES {
        r.issue(Error, "size", None, "ファイルは 1 MiB 以下にしてください。");
        return r;
    }
    r.info.bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
    let bytes = if r.info.bom { &bytes[3..] } else { bytes };
    let Ok(text) = std::str::from_utf8(bytes) else {
        r.issue(
            Error,
            "utf8",
            None,
            "UTF-8 として読めません。エディタで UTF-8 に保存してから再検査してください。",
        );
        return r;
    };
    if text.is_empty() {
        r.issue(Error, "empty", None, "解答テキストを入力してください。");
        return r;
    }
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\r' if bytes.get(pos + 1) == Some(&b'\n') => {
                r.info.crlf += 1;
                pos += 1;
            }
            b'\r' => r.info.cr += 1,
            b'\n' => r.info.lf += 1,
            _ => {}
        }
        pos += 1;
    }
    if r.info.bom {
        r.issue(
            Warning,
            "bom",
            Some(1),
            "UTF-8 BOM が付いています。修正案では取り除きます。",
        );
        r.changes.push("UTF-8 BOM を削除");
    }
    let kinds = [r.info.lf, r.info.crlf, r.info.cr]
        .iter()
        .filter(|&&n| n > 0)
        .count();
    if kinds > 1 {
        r.issue(
            Warning,
            "mixed-newline",
            None,
            "改行コードが混在しています。修正案は LF に統一します。",
        );
    } else if r.info.crlf + r.info.cr > 0 {
        r.issue(
            Info,
            "newline",
            None,
            "改行は LF に統一します。講義資料に LF 必須の指定はありません。",
        );
    }
    if r.info.crlf + r.info.cr > 0 {
        r.changes.push("改行コードを LF に統一");
    }
    if !text.ends_with('\n') && !text.ends_with('\r') {
        r.issue(
            Info,
            "final-newline",
            None,
            "修正案の末尾には改行を1つ付けます。",
        );
    } else if text.ends_with("\n\n") || text.ends_with("\r\n\r\n") || text.ends_with("\r\r") {
        r.issue(
            Warning,
            "final-newline",
            None,
            "末尾に余分な空行があります。",
        );
    }
    let lf_text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut bodies: Vec<Vec<AnswerLine<'_>>> = vec![Vec::new(); assignment.sections.len()];
    let mut has_answer = vec![false; assignment.sections.len()];
    let mut headers: [Option<AnswerLine<'_>>; 2] = [None, None];
    let mut active: Option<usize> = None;
    let mut next = 0;
    let mut seen = vec![false; assignment.sections.len()];
    let mut unambiguous = true;
    let mut comment_count = 0;
    for (i, line) in lf_text.split_terminator('\n').enumerate() {
        let number = i + 1;
        let row = AnswerLine { text: line, number };
        let hash_final = assignment.hash_prefixed_finals
            && hash_final_state_candidate(line)
            && active.is_some_and(|index| {
                assignment.sections[index].format == Format::DfaTransitions && !has_answer[index]
            });
        if is_comment(line) && !hash_final {
            comment_count += 1;
            r.issue(
                Warning,
                "comment",
                Some(number),
                "# で始まるコメントが残っています。修正案とハッシュ対象から除きます。",
            );
            continue;
        }
        if let Some(index) = assignment.sections.iter().position(|s| s.marker == line) {
            if active.is_some() || seen[index] || index != next {
                r.issue(
                    Error,
                    "section-order",
                    Some(number),
                    "設問の重複・順序違い、または直前の終了区切りの欠落があります。",
                );
                unambiguous = false;
            }
            seen[index] = true;
            next = index + 1;
            active = Some(index);
        } else if line == "========" {
            if active.take().is_none() {
                r.issue(
                    Error,
                    "delimiter",
                    Some(number),
                    "対応する設問開始行がない終了区切りです。",
                );
                unambiguous = false;
            }
        } else if looks_like_marker(line.trim())
            || (!line.trim().is_empty() && line.trim().chars().all(|c| c == '='))
        {
            r.issue(Error, "delimiter", Some(number), "設問番号や = の個数・空白がテンプレートと一致しません。選択した課題も確認してください。");
            unambiguous = false;
        } else if let Some(index) = active {
            has_answer[index] |= !line.trim().is_empty();
            bodies[index].push(row);
        } else if let Some(h) = ["学籍番号:", "氏名:"]
            .iter()
            .position(|prefix| line.starts_with(prefix))
        {
            if headers[h].is_some() {
                r.issue(
                    Error,
                    "header-duplicate",
                    Some(number),
                    "氏名または学籍番号の行が重複しています。",
                );
                unambiguous = false;
            }
            let prefix = ["学籍番号:", "氏名:"][h];
            if line[prefix.len()..].trim().is_empty() {
                r.issue(
                    Error,
                    "header-empty",
                    Some(number),
                    format!("{}が未記入です。", prefix.trim_end_matches(':')),
                );
            }
            headers[h] = Some(row);
        } else if !line.trim().is_empty() {
            r.issue(Error, "outside-section", Some(number), "解答欄外に未認識の文字があります。氏名・学籍番号のラベルはテンプレートどおりにしてください。");
            unambiguous = false;
        }
    }
    if active.is_some() {
        r.issue(
            Error,
            "delimiter",
            None,
            "最後の解答欄の終了区切り ======== がありません。",
        );
        unambiguous = false;
    }
    for (h, header) in headers.iter().enumerate() {
        if header.is_none() {
            r.issue(
                Error,
                "header-missing",
                None,
                format!("{}の行がありません。", ["学籍番号:", "氏名:"][h]),
            );
        }
    }
    let mut blocks = Vec::new();
    let mut trimmed = false;
    let mut removed_blanks = false;
    let mut sorted = false;
    for (index, spec) in assignment.sections.iter().enumerate() {
        if !seen[index] {
            r.issue(
                Error,
                "section-missing",
                None,
                format!(
                    "設問 {} がありません。選択した課題を確認してください。",
                    spec.id
                ),
            );
            unambiguous = false;
            continue;
        }
        let rows = &bodies[index];
        for row in rows {
            check_row(&mut r, spec, row);
        }
        let nonempty: Vec<_> = rows
            .iter()
            .filter(|row| !row.text.trim().is_empty())
            .collect();
        if nonempty.is_empty() {
            r.issue(
                Error,
                "unanswered",
                rows.first().map(|r| r.number),
                format!("{} の解答欄が空です。", spec.id),
            );
        }
        if let Some(validate) = spec.validate {
            validate(&mut r, &nonempty);
        }
        let first = rows
            .iter()
            .position(|row| !edge_trim(row.text).is_empty())
            .unwrap_or(rows.len());
        let last = rows
            .iter()
            .rposition(|row| !edge_trim(row.text).is_empty())
            .map(|i| i + 1)
            .unwrap_or(first);
        let edge_blanks = first > 0 || last < rows.len();
        if edge_blanks {
            removed_blanks = true;
            r.issue(
                Warning,
                "edge-blank",
                rows.first().map(|row| row.number),
                format!(
                    "{} の解答欄の先頭・末尾に空行があります。修正案では除きます。",
                    spec.id
                ),
            );
        }
        let mut normalized_rows = Vec::new();
        for row in &rows[first..last] {
            let value = if spec.format == Format::Prose {
                row.text
            } else {
                edge_trim(row.text)
            };
            trimmed |= value != row.text;
            if value.is_empty()
                && (matches!(
                    spec.format,
                    Format::Compact | Format::Productions | Format::PngFilename
                ) || (spec.format == Format::DfaTransitions && normalized_rows.len() > 1))
            {
                removed_blanks = true;
                r.issue(
                    Warning,
                    "inner-blank",
                    Some(row.number),
                    "解答の列挙に空行が含まれます。修正案では除きます。",
                );
            } else {
                normalized_rows.push(value);
            }
        }
        if spec.format == Format::Productions {
            let before = normalized_rows.clone();
            normalized_rows.sort_unstable(); // Rust string order = ASCII byte order here, no locale collation.
            sorted |= normalized_rows != before;
        }
        let body = if spec.format == Format::DfaTransitions && !normalized_rows.is_empty() {
            let mut finals: Vec<_> = normalized_rows[0].split(',').collect();
            let before_finals = finals.clone();
            finals.sort_unstable();
            sorted |= before_finals != finals;
            let mut transitions: Vec<_> = normalized_rows
                .iter()
                .skip(1)
                .copied()
                .filter(|s| !s.is_empty())
                .collect();
            let before = transitions.clone();
            transitions.sort_unstable();
            sorted |= before != transitions;
            if transitions.is_empty() {
                finals.join(",")
            } else {
                format!("{}\n\n{}", finals.join(","), transitions.join("\n"))
            }
        } else {
            normalized_rows.join("\n")
        };
        blocks.push(if body.is_empty() {
            format!("{}\n========", spec.marker)
        } else {
            format!("{}\n{}\n========", spec.marker, body)
        });
    }
    if comment_count > 0 {
        r.changes.push("# で始まるコメント行を削除");
    }
    if trimmed {
        r.changes
            .push("文章欄以外の行頭・行末の半角空白とタブを削除");
    }
    if removed_blanks {
        r.changes.push("解答欄の端の空行・列挙欄内の空行を削除");
    }
    if sorted {
        r.changes
            .push("生成規則・最終状態・遷移を ASCII の辞書順に並べ替え（導出の順序は保持）");
    }
    if unambiguous {
        let hash_input = format!("{}\n", blocks.join("\n\n"));
        let mut header_text = headers
            .iter()
            .flatten()
            .map(|row| row.text)
            .collect::<Vec<_>>()
            .join("\n");
        if !header_text.is_empty() {
            header_text.push_str("\n\n");
        }
        let normalized = format!("{header_text}{hash_input}");
        if normalized != lf_text && r.changes.is_empty() {
            r.changes.push("欄間の空行と末尾改行を統一");
        }
        r.hash = Some(sha256(&hash_input));
        r.hash_input = Some(hash_input);
        r.normalized = Some(normalized);
    }
    r
}

fn check_row(r: &mut Report, spec: &Section, row: &AnswerLine<'_>) {
    use Severity::*;
    if spec.ascii && !row.text.is_ascii() {
        r.issue(
            Error,
            "ascii",
            Some(row.number),
            format!("{} の解答には ASCII 以外の文字が含まれています。", spec.id),
        );
    }
    if row.text != edge_trim(row.text) {
        let action = if spec.format == Format::Prose {
            "文章欄のため自動削除しません。"
        } else {
            "修正案では除きます。"
        };
        r.issue(
            Warning,
            "edge-space",
            Some(row.number),
            format!("行頭・行末に半角空白またはタブがあります。{action}"),
        );
    }
    if row.text.contains('\t') {
        r.issue(
            Warning,
            "tab",
            Some(row.number),
            "タブが含まれています。行中のタブは自動変更しません。",
        );
    }
    if row.text.contains('　') {
        r.issue(
            Warning,
            "wide-space",
            Some(row.number),
            "全角空白が含まれています。自動変更しません。",
        );
    }
    if row.text.chars().any(|c| c.is_control() && c != '\t' || matches!(c, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}' | '\u{00a0}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')) {
        r.issue(Error, "invisible", Some(row.number), "制御文字・不可視文字が含まれています。文字を確認して取り除いてください。");
    }
    if matches!(
        spec.format,
        Format::Compact | Format::Productions | Format::DfaTransitions
    ) && edge_trim(row.text).contains(' ')
    {
        r.issue(
            Warning,
            "inner-space",
            Some(row.number),
            "列挙・記号の行中に半角空白があります。必要な文字の可能性があるため自動削除しません。",
        );
    }
}

pub fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
