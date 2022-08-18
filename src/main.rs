use cargo_upgrades::*;

fn main() {
    // When run via Cargo, the command name is present
    let args: Vec<_> = std::env::args().collect();

    let mut opts = getopts::Options::new();
    opts.optflag("", "pre", "Suggest upgrades from stable to pre-release (alpha, beta) versions");
    opts.optopt("", "manifest-path", "Check this Cargo project instead of the current dir", "Cargo.toml");
    opts.optflag("h", "help", "This help");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {}\n{}", e, opts.usage("https://gitlab.com/kornelski/cargo-upgrades"));
            std::process::exit(1);
        },
    };

    if matches.opt_present("h") {
        println!("cargo-upgrades v{}\n{}", env!("CARGO_PKG_VERSION"), opts.usage("https://gitlab.com/kornelski/cargo-upgrades"));
        std::process::exit(0);
    }

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
                "\t{} matches {};\tlatest is {}",
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
        println!("OK! Cargo.toml allows `cargo update` to use latest dependencies.");
    }
}
