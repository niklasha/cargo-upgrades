use cargo_upgrades::*;

fn main() {
    // When run via Cargo, the command name is present
    let args: Vec<_> = std::env::args().collect();

    let mut opts = getopts::Options::new();
    opts.optopt("", "manifest-path", "Alternative location", "Cargo.toml");
    opts.optflag("", "pre", "Allow pre-release versions");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("cargo-upgrades v{}\n{}\nUsage: {} --manifest-path=Cargo.toml", env!("CARGO_PKG_VERSION"), e, args[0]);
            std::process::exit(1);
        },
    };

    let pre = matches.opt_present("pre");
    let manifest_path = matches.opt_str("manifest-path");
    let u = match UpgradesCheckerInit::new(manifest_path.as_deref()) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        },
    };
    let u = match u.checker() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        },
    };

    let mut printed_anything = false;
    for (package, deps) in u.outdated_dependencies(pre) {
        if printed_anything {
            println!();
        }
        println!("{}: {}", package.name, package.manifest_path);
        for d in deps {
            let matches = d.matches.map(|s| s.to_string());
            let dep = d.dependency;
            println!(
                "  {} matches {}; latest is {}",
                dep.name,
                matches.as_deref().unwrap_or("nothing"),
                d.latest
            );
        }
        printed_anything = true;
    }

    if printed_anything {
        std::process::exit(7);
    } else {
        println!("All dependencies are up to date!");
    }
}
