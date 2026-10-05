use answer_checker::check;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: cargo run -p answer-checker --example inspect -- <01|02|03> <file.txt>");
        std::process::exit(2);
    }
    let bytes = std::fs::read(&args[2]).expect("read input file");
    let report = check(&args[1], &bytes);
    println!(
        "errors={} warnings={} hash_available={}",
        report.errors(),
        report.warnings(),
        report.hash.is_some()
    );
    for issue in report.issues {
        println!(
            "{:?} line={:?} {}: {}",
            issue.severity, issue.line, issue.code, issue.message
        );
    }
}
