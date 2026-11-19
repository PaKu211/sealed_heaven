//! Qinghe client. The default view is ASCII. `--window` opens the Bevy view.
mod agency_desktop;

fn main() {
    if std::env::args().any(|arg| arg == "--window") {
        agency_desktop::run();
    } else {
        sealed_playable::ascii::run();
    }
}
