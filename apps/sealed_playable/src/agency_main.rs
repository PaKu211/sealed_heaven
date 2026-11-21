//! Qinghe's turn-based ASCII client, with previous milestones available explicitly.
#[cfg(feature = "desktop")]
mod agency_desktop;

fn main() {
    let options: Vec<_> = std::env::args().skip(1).collect();
    if options.iter().any(|arg| arg == "--pixel") {
        #[cfg(feature = "desktop")]
        sealed_playable::ascii::run_regional_window();
        #[cfg(not(feature = "desktop"))]
        eprintln!("The window requires the desktop feature.");
    } else if options.iter().any(|arg| arg == "--window") {
        #[cfg(feature = "desktop")]
        agency_desktop::run();
        #[cfg(not(feature = "desktop"))]
        eprintln!("The window requires the desktop feature.");
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
    } else if options.iter().any(|arg| arg == "--c22") {
        sealed_playable::ascii::run_c22();
    } else if options.iter().any(|arg| arg == "--debug-world") {
        sealed_playable::ascii::run_developer_view();
    } else {
        sealed_playable::ascii::run();
    }
}
