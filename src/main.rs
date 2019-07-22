use cargo_upgrades::*;
use clap::*;

fn main() {
    // When run via Cargo, the command name is present
    let args = std::env::args().enumerate().filter(|&(i,ref a)| {
        i != 1 || a != "upgrades"
    }).map(|(_,a)| a);

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("manifest-path")
                .long("manifest-path")
                .value_name("Cargo.toml")
                .takes_value(true),
        )
        .get_matches_from(args);

    let manifest_path = matches.value_of("manifest-path");
    let u = match UpgradesChecker::new(manifest_path) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let mut printed_anything = false;
    for (package, deps) in u.outdated_dependencies() {
        if printed_anything {
            println!();
        }
        println!("{}: {}", package.name, package.manifest_path.display());
        for d in deps {
            let matches = d.matches.map(|s| s.to_string());
            let dep = d.dependency;
            let req = dep.req.to_string();
            println!("  {} {} matches {}; latest is {}", dep.name, req.trim_start_matches('^'), matches.as_ref().map(|s| s.as_str()).unwrap_or("nothing"), d.latest);
        }
        printed_anything = true;
    }

    if printed_anything {
        std::process::exit(7);
    } else {
        println!("All dependencies are up to date!");
    }
}
