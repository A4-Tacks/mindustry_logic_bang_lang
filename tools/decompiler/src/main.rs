use std::{env::args, fs, io::{self, stdin}, process::exit, time::SystemTime, rc::Rc};

use mlog_decompiler::{Finder, Reduce, clean, make, quality::Loss, walk};
use getopts_macro::getopts_options;
use tag_code::logic_parser;

struct Config {
    raw_out: bool,
    dirty_out: bool,
}

fn main() {
    let options = getopts_options! {
        -g, --guidance      "guidance mode";
        -i, --iterate=N     "maximum number of iterations";
        -l, --limit=N       "maximum case limit for each iteration";
        -L, --out-limit=N   "maximum output limit for finished case";
        -r, --raw-out       "use raw-format (logic-style) outputs";
        -d, --dirty-out     "use non clean outputs";
        -s, --sparse        "sparse cases output";
        -h, --help*         "show help messages";
        -v, --version       "show version";
    };
    let matched = match options.parse(args().skip(1)) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("{e}");
            exit(2)
        },
    };
    if matched.opt_present("help") {
        let desc = env!("CARGO_PKG_DESCRIPTION");
        let repo = env!("CARGO_PKG_REPOSITORY");

        println!("Usage: {} [Options] [FILE]..", env!("CARGO_BIN_NAME"));
        println!("{}", options.usage(desc).trim_end());
        println!();
        println!("{EXTRA}");
        println!();
        println!("repo: {repo}");
        return;
    }
    if matched.opt_present("version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let guidance = matched.opt_present("guidance");
    let raw_out = matched.opt_present("raw-out");
    let dirty_out = matched.opt_present("dirty-out");
    let sparse = matched.opt_present("sparse");
    let iterate: usize = matched.opt_get("iterate")
        .expect("invalid iterate arg")
        .unwrap_or(30);
    let limit: usize = matched.opt_get("limit")
        .expect("invalid limit arg")
        .unwrap_or(300);
    let out_limit: usize = matched.opt_get("out-limit")
        .expect("invalid out-limit arg")
        .unwrap_or(1);

    let cfg = Config { raw_out, dirty_out };

    let input = if matched.free.is_empty() {
        io::read_to_string(stdin().lock()).unwrap()
    } else {
        matched.free.iter()
            .map(|path|
        {
            match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("cannot read `{path}`: {e}");
                    exit(1)
                },
            }
        }).collect()
    };

    let mut lines = match logic_parser::parser::lines(&input) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("parse logic error at {}, expected {}", e.location, e.expected);
            eprintln!(" ->\t{}", input.lines()
                .nth(e.location.line.saturating_sub(1))
                .unwrap_or("<unknown line>"));
            exit(1)
        },
    };
    lines.index_label_popup();
    lines.dup_label_pairs();

    let lines = lines.lines()
        .iter()
        .map(|it| &it.value);
    let reduces = make::make_reduce(lines);

    let mut finder = Finder {
        current: FromIterator::from_iter([reduces.into_iter().collect()]),
        losses_cache: vec![],
        limit,
        guidance,
    };

    eprintln!("iterate    {iterate}\nlimit      {limit}\nout-limit  {out_limit}");

    let start_time = SystemTime::now();
    let mut time = start_time.clone();
    let mut prev_limite = None;
    let [mut itering, mut iterate] = [1, iterate];
    loop {
        eprint!("{itering:>3}/{iterate:<3} ");
        finder.iterate();
        let mem_usage = mem_usage();

        let raw_len = finder.current.len();
        eprint!("limite {:>8}", raw_len);

        let (happy, limite) = finder.limite();
        eprint!(" -> {:<8} <{happy:.5} $ {limite:.5}>", finder.current.len());
        eprint!(" {:.2}s", time.elapsed().unwrap().as_secs_f64());

        if let Some(usage) = mem_usage {
            let mb = usage >> 20;
            if mb > 2048 {
                eprint!(" (Used {mb} MiB of memory)")
            }
        }

        eprintln!();

        if Some((happy, limite, raw_len)) == prev_limite {
            eprintln!("-- Early Reconstruction Completed");
            break;
        }

        time = SystemTime::now();
        prev_limite = Some((happy, limite, raw_len));

        itering += 1;
        if itering > iterate {
            if !atty::is(atty::Stream::Stdin) || !atty::is(atty::Stream::Stderr) {
                break
            }
            eprint!("There may be better results, should we continue? [Y/n]");
            let buf = &mut String::new();
            let _ = stdin().read_line(buf);
            let ("" | "y" | "Y") = buf.trim() else { break };
            iterate += iterate.div_ceil(2);
            eprint!("\r");
        }
    }
    eprintln!("-- Reconstruction Completed, Elapsed: {:.4}s", start_time.elapsed().unwrap().as_secs_f64());

    let mut sorted = finder.current.iter().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.loss().total_cmp(&b.loss()));

    if sparse {
        let step = sorted.len() / out_limit.max(1);
        output(cfg, sorted.iter()
            .copied()
            .enumerate()
            .step_by(step)
            .take(out_limit))
    } else {
        output(cfg, sorted.iter()
            .copied()
            .enumerate()
            .take(out_limit))
    }
}

fn output<'a>(
    Config { raw_out, dirty_out }: Config,
    iter: impl IntoIterator<Item = (usize, &'a Rc<[Reduce<'a>]>)>,
) {
    for (i, reduces) in iter {
        let loss = reduces.loss();
        let reduce = reduces.iter().cloned().collect::<Reduce<'_>>();

        let result = if !dirty_out {
            let cleaned = clean::dedup_labels(reduce);
            let cleaned = clean::jump_to_break(cleaned);
            let cleaned = clean::unused_labels(cleaned);
            cleaned
        } else {
            reduce
        };

        let def = walk::label_defs(&result);
        let used = walk::label_usages(&result);

        println!("#\x1b[1;92m---------- reduce[{def}/{used}] case {i} <{loss}> ----------\x1b[0m");

        if raw_out {
            println!("{result}");
        } else {
            println!("{result:x}");
        }
    }
}

fn mem_usage() -> Option<u64> {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_all();

    let pid = sysinfo::get_current_pid().ok()?;

    sys.process(pid)?.memory().into()
}

const EXTRA: &str = "\
Set a larger number of iterations and upper limit to improve quality

Higher limits use more memory and are slower,
If the input is long, you can set more iterations and fewer limits";
