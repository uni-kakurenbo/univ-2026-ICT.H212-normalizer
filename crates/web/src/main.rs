use answer_checker::{HASH_VERSION, MAX_BYTES, Severity, assignments::ASSIGNMENTS, check};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
struct CopyProps {
    text: String,
    label: &'static str,
}

#[component]
fn CopyButton(props: &CopyProps) -> Html {
    let status = use_state(String::new);
    let onclick = {
        let text = props.text.clone();
        let status = status.clone();
        Callback::from(move |_| {
            let text = text.clone();
            let status = status.clone();
            // Start the clipboard operation synchronously in the click handler.
            let clipboard = web_sys::window()
                .expect("browser window")
                .navigator()
                .clipboard();
            let value: &JsValue = clipboard.as_ref();
            if value.is_undefined() || value.is_null() {
                status.set("コピーできません。欄内を選択してコピーしてください".into());
                return;
            }
            let promise = clipboard.write_text(&text);
            spawn_local(async move {
                status.set(
                    if JsFuture::from(promise).await.is_ok() {
                        "コピーしました"
                    } else {
                        "コピーできません。欄内を選択してコピーしてください"
                    }
                    .to_string(),
                );
            });
        })
    };
    html! { <div class="copy-control"><button class="button secondary" {onclick}>{props.label}</button><span role="status">{&*status}</span></div> }
}

#[derive(Properties, PartialEq)]
struct DownloadProps {
    text: String,
    filename: String,
}

#[component]
fn DownloadLink(props: &DownloadProps) -> Html {
    // Byte-backed Blob preserves LF and UTF-8 exactly. ObjectUrl revokes the
    // temporary URL when this preview changes or the component is removed.
    let url = use_memo(props.text.clone(), |text| {
        gloo_file::ObjectUrl::from(gloo_file::Blob::new_with_options(
            text.as_bytes(),
            Some("text/plain;charset=utf-8"),
        ))
    });
    html! { <a class="button secondary download-link" href={url.to_string()} download={props.filename.clone()}>{".txt をダウンロード"}</a> }
}

#[component]
fn App() -> Html {
    let assignment = use_state(|| "01".to_string());
    let input = use_state(String::new);
    let bytes = use_state(Vec::<u8>::new);
    let origin = use_state(|| "貼り付け".to_string());
    let checked = use_state(|| false);
    let loading = use_state(|| false);
    let load_error = use_state(String::new);
    let revision = use_mut_ref(|| 0u64);
    let file_ref = use_node_ref();

    let on_assignment = {
        let assignment = assignment.clone();
        Callback::from(move |event: Event| {
            assignment.set(event.target_unchecked_into::<HtmlSelectElement>().value())
        })
    };
    let on_input = {
        let input = input.clone();
        let bytes = bytes.clone();
        let origin = origin.clone();
        let checked = checked.clone();
        let revision = revision.clone();
        let loading = loading.clone();
        let load_error = load_error.clone();
        Callback::from(move |event: InputEvent| {
            *revision.borrow_mut() += 1;
            let value = event.target_unchecked_into::<HtmlTextAreaElement>().value();
            bytes.set(value.as_bytes().to_vec());
            input.set(value);
            origin.set("貼り付け・編集".into());
            checked.set(false);
            loading.set(false);
            load_error.set(String::new());
        })
    };
    let on_file = {
        let input = input.clone();
        let bytes = bytes.clone();
        let origin = origin.clone();
        let checked = checked.clone();
        let loading = loading.clone();
        let load_error = load_error.clone();
        let revision = revision.clone();
        Callback::from(move |event: Event| {
            let element = event.target_unchecked_into::<HtmlInputElement>();
            let Some(file) = element.files().and_then(|files| files.get(0)) else {
                return;
            };
            element.set_value(""); // Permit choosing the same file after saving it again.
            *revision.borrow_mut() += 1;
            let request = *revision.borrow();
            if file.size() > MAX_BYTES as f64 {
                load_error.set("ファイルは 1 MiB 以下にしてください。".into());
                return;
            }
            loading.set(true);
            checked.set(false);
            load_error.set(String::new());
            let input = input.clone();
            let bytes = bytes.clone();
            let origin = origin.clone();
            let checked = checked.clone();
            let loading = loading.clone();
            let load_error = load_error.clone();
            let revision = revision.clone();
            spawn_local(async move {
                let name = file.name();
                let blob = gloo_file::Blob::from(file);
                let result = gloo_file::futures::read_as_bytes(&blob).await;
                if *revision.borrow() != request {
                    return;
                }
                loading.set(false);
                match result {
                    Ok(raw) => {
                        // Raw bytes remain authoritative. Lossy decoding is used ONLY
                        // to show invalid input, never for validation or hashing.
                        input.set(
                            String::from_utf8_lossy(&raw)
                                .trim_start_matches('\u{feff}')
                                .to_string(),
                        );
                        bytes.set(raw);
                        origin.set(format!("ファイル：{name}"));
                        checked.set(true);
                    }
                    Err(_) => {
                        load_error.set(
                            "ファイルを読み込めませんでした。もう一度選択してください。".into(),
                        );
                    }
                }
            });
        })
    };
    let choose_file = {
        let file_ref = file_ref.clone();
        Callback::from(move |_| {
            if let Some(element) = file_ref.cast::<HtmlInputElement>() {
                element.click();
            }
        })
    };
    let inspect = {
        let checked = checked.clone();
        Callback::from(move |_| checked.set(true))
    };
    let clear = {
        let input = input.clone();
        let bytes = bytes.clone();
        let checked = checked.clone();
        let origin = origin.clone();
        let revision = revision.clone();
        let loading = loading.clone();
        let load_error = load_error.clone();
        Callback::from(move |_| {
            *revision.borrow_mut() += 1;
            input.set(String::new());
            bytes.set(Vec::new());
            checked.set(false);
            origin.set("貼り付け".into());
            loading.set(false);
            load_error.set(String::new());
        })
    };
    let report = check(&assignment, &bytes);
    let spec = ASSIGNMENTS
        .iter()
        .find(|a| a.id == *assignment)
        .expect("registered assignment");
    let apply_preview = {
        let value = report.normalized.clone().unwrap_or_default();
        let input = input.clone();
        let bytes = bytes.clone();
        let origin = origin.clone();
        let checked = checked.clone();
        Callback::from(move |_| {
            bytes.set(value.as_bytes().to_vec());
            input.set(value.clone());
            origin.set("修正案を再検査".into());
            checked.set(true);
        })
    };
    let errors = report.errors();
    let warnings = report.warnings();
    let preview_errors = report
        .normalized
        .as_ref()
        .map(|text| check(&assignment, text.as_bytes()).errors())
        .unwrap_or(0);

    html! {
        <>
        <header class="site-header"><div class="brand"><span class="brand-mark" aria-hidden="true">{"[=]"}</span><span>{"AUTOLANG"}</span></div><span class="header-meta">{"ICT.H212 · 2026"}</span></header>
        <main>
            <div class="page-heading"><div><p class="eyebrow">{"オートマトンと言語 (情報通信)"}</p><h1>{"書式チェック"}</h1></div></div>
            <section class="workspace" aria-label="解答の入力と検査">
                <div class="input-pane">
                    <div class="section-label"><span class="step">{"01"}</span><h2>{"課題と解答"}</h2></div>
                    <label class="field-label" for="assignment">{"課題を選択"}</label>
                    <select id="assignment" onchange={on_assignment}>{for ASSIGNMENTS.iter().map(|a| html! { <option value={a.id} selected={a.id == *assignment}>{a.title}</option> })}</select>
                    <p class="assignment-note">{spec.note}</p>
                    <div class="input-toolbar"><label class="field-label" for="answer">{"解答テキスト"}</label><button class="text-button" onclick={choose_file} disabled={*loading}>{".txt を読み込む"}</button><input ref={file_ref} class="hidden-input" type="file" accept=".txt,text/plain" onchange={on_file} aria-label="解答ファイルを選択" /></div>
                    <textarea id="answer" class="answer-input" value={(*input).clone()} oninput={on_input} spellcheck="false" placeholder={format!("学籍番号: …\n氏名: …\n\n{}\n解答\n========\n…", spec.sections[0].marker)} aria-describedby="input-help" />
                    <div class="input-meta"><span>{if *loading { "読み込み中…" } else { &*origin }}</span><span>{format!("{} bytes", bytes.len())}</span></div>
                    <p id="input-help" class="help">{"元ファイルの文字コード・改行を調べるには .txt を読み込んでください。貼り付け後は元の保存形式を判定できません。"}</p>
                    if !load_error.is_empty() { <p class="load-error" role="alert">{&*load_error}</p> }
                    <div class="actions"><button class="button primary" onclick={inspect} disabled={*loading}>{"形式をチェック"}</button><button class="text-button" onclick={clear}>{"クリア"}</button></div>
                </div>
                <div class="result-pane">
                    <div class="section-label"><span class="step">{"02"}</span><h2>{"検査結果"}</h2></div>
                    if !*checked {
                        <div class="empty-result"><span class="empty-symbol" aria-hidden="true">{"{ }"}</span><h3>{"解答を入力して検査"}</h3><p>{"区切り、文字、空白、改行を確認し、問題のある行を表示します。"}</p></div>
                    } else {
                        <div class={classes!("result-summary", if errors > 0 { "has-errors" } else if warnings > 0 { "has-warnings" } else { "is-valid" })} role="status">
                            <h3>{if errors > 0 { "修正が必要な形式があります" } else if warnings > 0 { "形式エラーなし・確認事項あり" } else { "検査対象の形式に問題はありません" }}</h3>
                            <div class="counts"><span>{format!("エラー {errors}")}</span><span>{format!("注意 {warnings}")}</span></div>
                        </div>
                        <div class="file-facts"><span>{format!("LF {}", report.info.lf)}</span><span>{format!("CRLF {}", report.info.crlf)}</span><span>{format!("CR {}", report.info.cr)}</span><span>{if report.info.bom { "BOM あり" } else { "BOM なし" }}</span></div>
                        <ol class="diagnostics">{for report.issues.iter().map(|issue| {
                            let (class, label) = match issue.severity { Severity::Error => ("error", "エラー"), Severity::Warning => ("warning", "注意"), Severity::Info => ("info", "情報") };
                            html! { <li class={class}><div class="issue-meta"><span class="severity">{label}</span><span>{issue.line.map(|line| format!("{line} 行目")).unwrap_or_else(|| "ファイル全体".into())}</span></div><p>{&issue.message}</p></li> }
                        })}</ol>
                        <p class="help">{"これは入力の検査結果です。修正案に反映できる項目も含みます。解答の正しさを保証する表示ではありません。"}</p>
                    }
                </div>
            </section>
            if *checked {
                <section class="output-section" aria-label="修正案とハッシュ">
                    <div class="preview-pane"><div class="section-label"><span class="step">{"03"}</span><h2>{"修正プレビュー"}</h2></div>
                    if let Some(normalized) = &report.normalized {
                        <p class="help">{"氏名・学籍番号を含む提出用テキストです。変更を確認してからコピーしてください。"}</p>
                        <ul class="changes">{for report.changes.iter().map(|change| html! { <li>{change}</li> })}<li>{"欄間は空行1つ、末尾は LF 1つ"}</li></ul>
                        <textarea class="preview-text" aria-label="修正後の解答テキスト" readonly=true value={normalized.clone()} spellcheck="false" />
                        <p class="help">{format!("修正案に残っている形式エラー：{preview_errors} 件")}</p>
                        <div class="preview-actions"><CopyButton key={normalized.clone()} text={normalized.clone()} label="修正案をコピー" /><DownloadLink text={normalized.clone()} filename={format!("{}_answer-sheet-normalized.txt", *assignment)} /><button class="text-button" onclick={apply_preview}>{"修正案を再検査"}</button></div>
                        <p class="help">{"コピー後の保存設定で文字コード・改行が変わることがあります。提出直前の .txt を再読込して確認できます。"}</p>
                    } else { <p class="blocked-output">{"解答欄を確定できません。文字コード・設問・区切りのエラーを先に修正してください。"}</p> }
                    </div>
                    <div class="hash-pane"><div class="section-label"><span class="step">{"04"}</span><h2>{"解答の SHA-256"}</h2></div>
                    <p class="help">{"修正案から氏名・学籍番号・コメントを除き、設問番号と区切りを残して計算します。"}</p>
                    if let (Some(hash), Some(hash_input)) = (&report.hash, &report.hash_input) {
                        <label class="field-label" for="hash">{"ハッシュ値"}</label><textarea id="hash" class="hash-value" readonly=true value={hash.clone()} rows="3" spellcheck="false" />
                        <CopyButton key={hash.clone()} text={hash.clone()} label="SHA-256 をコピー" />
                        <div class="hash-format"><span>{HASH_VERSION}</span><span>{"UTF-8 / LF / BOM なし"}</span><span>{format!("{} bytes · 末尾 LF 1つ", hash_input.len())}</span></div>
                        if preview_errors > 0 { <p class="hash-caution">{"形式エラーが残っている修正案のハッシュです。必要な修正を済ませて再検査してください。"}</p> }
                        <details class="hash-details"><summary>{"ハッシュ対象のテキストを確認"}</summary><textarea class="preview-text" readonly=true value={hash_input.clone()} aria-label="ハッシュ計算対象" spellcheck="false" /><CopyButton key={hash_input.clone()} text={hash_input.clone()} label="ハッシュ対象をコピー" /></details>
                    } else { <p class="blocked-output">{"解答欄を確定できるまで、ハッシュを計算しません。"}</p> }
                    </div>
                </section>
            }
            <footer><p>{"端末内のみで処理。入力をブラウザに保存しません。ダウンロードは操作したときだけ行います。"}</p><a href="https://github.com/uni-kakurenbo/univ-2026-ICT.H212-normalizer" target="_blank" rel="noopener noreferrer">{"ソースコード・検査仕様"}</a></footer>
        </main>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
