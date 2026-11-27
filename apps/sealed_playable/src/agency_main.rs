//! Qinghe's turn-based ASCII client, with previous milestones available explicitly.
#[cfg(feature = "desktop")]
mod agency_desktop;
mod worldgen_cli;

fn main() {
    let options: Vec<_> = std::env::args().skip(1).collect();
    if options.iter().any(|arg| arg == "--content-info") {
        if options.len() != 1 {
            eprintln!("--content-info must be used alone");
            std::process::exit(2);
        }
        println!("{}", sealed_sim::living::content::manifest_json());
        return;
    }
    if options.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--worldgen-info"
                | "--expansion-info"
                | "--c40"
                | "--c41"
                | "--c42"
                | "--c43"
                | "--c44"
                | "--c45"
                | "--c46"
        )
    }) {
        let result = if options.iter().any(|arg| arg == "--worldgen-info") {
            worldgen_cli::inspect(&options)
        } else if options.iter().any(|arg| arg == "--expansion-info") {
            worldgen_cli::inspect_expanded(&options)
        } else {
            worldgen_cli::launch(&options)
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(2);
        }
        return;
    }
    if options.iter().any(|arg| arg == "--pixel") {
        if options.len() != 1 {
            eprintln!("--pixel currently supports only the default C3.5.1 campaign.");
            std::process::exit(2);
        }
        #[cfg(feature = "desktop")]
        sealed_playable::ascii::run_regional_window();
        #[cfg(not(feature = "desktop"))]
        {
            eprintln!("The window requires the desktop feature.");
            std::process::exit(2);
        }
    } else if options.iter().any(|arg| arg == "--window") {
        #[cfg(feature = "desktop")]
        agency_desktop::run();
        #[cfg(not(feature = "desktop"))]
        {
            eprintln!("The window requires the desktop feature.");
            std::process::exit(2);
        }
    } else if options.iter().any(|arg| arg == "--c08") {
        sealed_playable::ascii::run_c08();
    } else if options.iter().any(|arg| arg == "--c09") {
        sealed_playable::ascii::run_c09();
    } else if options.iter().any(|arg| arg == "--c14") {
        sealed_playable::ascii::run_c14();
    } else if options.iter().any(|arg| arg == "--c15") {
        sealed_playable::ascii::run_c15();
    } else if options.iter().any(|arg| arg == "--c16") {
        sealed_playable::ascii::run_c16();
    } else if options.iter().any(|arg| arg == "--c17") {
        sealed_playable::ascii::run_c17();
    } else if options.iter().any(|arg| arg == "--c18") {
        sealed_playable::ascii::run_c18();
    } else if options.iter().any(|arg| arg == "--c19") {
        sealed_playable::ascii::run_c19();
    } else if options.iter().any(|arg| arg == "--c20") {
        sealed_playable::ascii::run_c20();
    } else if options.iter().any(|arg| arg == "--c21") {
        sealed_playable::ascii::run_c21();
    } else if options.iter().any(|arg| arg == "--c23") {
        sealed_playable::ascii::run_c23();
    } else if options.iter().any(|arg| arg == "--c24") {
        sealed_playable::ascii::run_c24();
    } else if options.iter().any(|arg| arg == "--c25") {
        sealed_playable::ascii::run_c25();
    } else if options.iter().any(|arg| arg == "--c27") {
        sealed_playable::ascii::run_c27();
    } else if options.iter().any(|arg| arg == "--c28") {
        sealed_playable::ascii::run_c28();
    } else if options.iter().any(|arg| arg == "--c29") {
        sealed_playable::ascii::run_c29();
    } else if options.iter().any(|arg| arg == "--c30") {
        sealed_playable::ascii::run_c30();
    } else if options.iter().any(|arg| arg == "--c31") {
        sealed_playable::ascii::run_c31();
    } else if options.iter().any(|arg| arg == "--c32") {
        sealed_playable::ascii::run_c32();
    } else if options.iter().any(|arg| arg == "--c33") {
        sealed_playable::ascii::run_c33();
    } else if options.iter().any(|arg| arg == "--c35") {
        sealed_playable::ascii::run_c35();
    } else if options.iter().any(|arg| arg == "--c34") {
        sealed_playable::ascii::run_c34();
    } else if options.iter().any(|arg| arg == "--c22") {
        sealed_playable::ascii::run_c22();
    } else if options.iter().any(|arg| arg == "--debug-world") {
        sealed_playable::ascii::run_developer_view();
    } else {
        sealed_playable::ascii::run();
    }
}
