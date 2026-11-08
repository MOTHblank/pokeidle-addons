mod collect;
mod export;
mod reduce;
mod trace;
#[path = "../../src/hunt_pacing_model.rs"]
mod hunt_pacing_model;

use reduce::ReduceConfig;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("hunt-pacing: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        print_usage();
        return Ok(());
    };

    match command {
        "collect" => {
            println!("{}", collect::instructions());
        }
        "reduce" => {
            if args.len() < 3 {
                return Err("reduce requires <raw-dir-or-file> <reduced.json>".into());
            }
            let config = parse_config(&args[3..])?;
            let sessions = trace::load_sessions(Path::new(&args[1]))?;
            let reduced = reduce::reduce_sessions(&sessions, config);
            export::write_reduced(Path::new(&args[2]), &reduced)?;
            println!(
                "reduced {} trace file(s), {} measured map(s) -> {}",
                reduced.trace_files, reduced.maps.len(), args[2]
            );
        }
        "export" => {
            if args.len() < 3 {
                return Err("export requires <reduced.json> <dataset.json> [--tsv <dataset.tsv>]".into());
            }
            let reduced = export::read_reduced(Path::new(&args[1]))?;
            export::write_dataset_json(Path::new(&args[2]), &reduced)?;
            if let Some(index) = args.iter().position(|arg| arg == "--tsv") {
                let path = args.get(index + 1)
                    .ok_or_else(|| "--tsv requires a path".to_owned())?;
                export::write_dataset_tsv(Path::new(path), &reduced)?;
            }
            println!("{}", export::coverage_report(&reduced));
        }
        "coverage" => {
            if args.len() != 2 {
                return Err("coverage requires <reduced.json>".into());
            }
            let reduced = export::read_reduced(Path::new(&args[1]))?;
            print!("{}", export::coverage_report(&reduced));
        }
        "validate" => {
            if args.len() != 2 {
                return Err("validate requires <reduced.json>".into());
            }
            let reduced = export::read_reduced(Path::new(&args[1]))?;
            print!("{}", export::validation_report(&reduced));
        }
        "all" => {
            if args.len() < 3 {
                return Err("all requires <raw-dir-or-file> <output-dir>".into());
            }
            let config = parse_config(&args[3..])?;
            let raw = Path::new(&args[1]);
            let out = PathBuf::from(&args[2]);
            fs::create_dir_all(&out)
                .map_err(|error| format!("could not create {}: {error}", out.display()))?;
            let sessions = trace::load_sessions(raw)?;
            let reduced = reduce::reduce_sessions(&sessions, config);
            let reduced_path = out.join("reduced.json");
            let dataset_path = out.join("moth-pacing.json");
            let tsv_path = out.join("moth-pacing.tsv");
            let coverage_path = out.join("coverage.txt");
            export::write_reduced(&reduced_path, &reduced)?;
            export::write_dataset_json(&dataset_path, &reduced)?;
            export::write_dataset_tsv(&tsv_path, &reduced)?;
            let coverage = export::coverage_report(&reduced);
            fs::write(&coverage_path, &coverage)
                .map_err(|error| format!("could not write {}: {error}", coverage_path.display()))?;
            print!("{coverage}");
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command: {other}")),
    }
    Ok(())
}

fn parse_config(args: &[String]) -> Result<ReduceConfig, String> {
    let mut config = ReduceConfig::default();
    let mut index = 0;
    while index < args.len() {
        let key = args[index].as_str();
        let value = args.get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        match key {
            "--warmup-kills" => config.warmup_kills = parse(value, key)?,
            "--min-kills" => config.min_measured_kills = parse(value, key)?,
            "--max-gap-ms" => config.max_packet_gap_ms = parse(value, key)?,
            "--min-spk" => config.min_seconds_per_kill = parse(value, key)?,
            "--max-spk" => config.max_seconds_per_kill = parse(value, key)?,
            "--mad" => config.outlier_mad_multiplier = parse(value, key)?,
            other => return Err(format!("unknown reducer option: {other}")),
        }
        index += 2;
    }
    if config.min_measured_kills == 0 {
        return Err("--min-kills must be > 0".into());
    }
    if config.max_packet_gap_ms == 0 {
        return Err("--max-gap-ms must be > 0".into());
    }
    if !(config.min_seconds_per_kill > 0.0
        && config.max_seconds_per_kill > config.min_seconds_per_kill)
    {
        return Err("seconds/kill bounds are invalid".into());
    }
    if config.outlier_mad_multiplier <= 0.0 {
        return Err("--mad must be > 0".into());
    }
    Ok(config)
}

fn parse<T: std::str::FromStr>(raw: &str, option: &str) -> Result<T, String> {
    raw.parse::<T>().map_err(|_| format!("invalid value for {option}: {raw}"))
}

fn print_usage() {
    println!(
r#"hunt-pacing — empirical PokéIdle server pacing reducer

Commands:
  hunt-pacing collect
  hunt-pacing reduce <raw-dir-or-file> <reduced.json> [reducer options]
  hunt-pacing export <reduced.json> <dataset.json> [--tsv <dataset.tsv>]
  hunt-pacing coverage <reduced.json>
  hunt-pacing validate <reduced.json>
  hunt-pacing all <raw-dir-or-file> <output-dir> [reducer options]

Reducer options:
  --warmup-kills N   discard N initial kills after every valid sequence boundary (default 5)
  --min-kills N      require N measured kill intervals per window (default 20)
  --max-gap-ms N     split on a packet gap above N ms (default 10000)
  --min-spk N        physical lower seconds/kill bound (default 0.20)
  --max-spk N        physical upper seconds/kill bound (default 120)
  --mad N            robust outlier threshold in MAD units (default 6)
"#
    );
}
