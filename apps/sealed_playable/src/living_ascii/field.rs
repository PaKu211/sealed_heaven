//! Presentation-only field menu. Commands use the canonical Session
//! parser. Hints use own state, direct sight and learned records only.
use super::{Session, actor_name, living_format};
use crate::locale_living::living_dynamic_text;
use sealed_sim::living::{
    ActorId, BREATH_MASTERY, Direction, PLAYER, Position,
    exploration::Landmark,
    favors::{Need as FavorNeed, Status as FavorStatus},
    requests::{Site, Status as RequestStatus},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub(crate) enum Page {
    #[default]
    Local,
    Travel,
    Training,
    Commitments,
    Journal,
    System,
}
impl Page {
    pub(crate) const ALL: [Self; 6] = [
        Self::Local,
        Self::Travel,
        Self::Training,
        Self::Commitments,
        Self::Journal,
        Self::System,
    ];
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Travel => "travel",
            Self::Training => "training",
            Self::Commitments => "commitments",
            Self::Journal => "journal",
            Self::System => "system",
        }
    }
    #[cfg(feature = "desktop")]
    pub(crate) fn shift(self, forward: bool) -> Self {
        let i = Self::ALL.iter().position(|&p| p == self).unwrap();
        Self::ALL[(i + if forward { 1 } else { Self::ALL.len() - 1 }) % Self::ALL.len()]
    }
}
#[derive(Default)]
pub(crate) struct FieldUi {
    pub(crate) page: Option<Page>,
    pub(crate) cursor: usize,
    pub(crate) reader: Option<usize>,
    pub(crate) atlas: bool,
    pub(crate) waypoint: Option<Landmark>,
}
impl FieldUi {
    pub(crate) fn close(&mut self) {
        self.page = None;
        self.reader = None;
        self.atlas = false;
        self.cursor = 0;
    }
    pub(crate) fn open(&mut self, page: Page) {
        self.close();
        self.page = Some(page);
    }
    pub(crate) fn modal(&self) -> bool {
        self.page.is_some() || self.reader.is_some() || self.atlas
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Entry {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) read: bool,
}
impl Session {
    pub(crate) fn field_text(&self, key: &str) -> &str {
        living_dynamic_text(self.locale, &format!("living.field.{key}"))
            .expect("validated field label")
    }
    pub(crate) fn field_fmt(&self, key: &str, params: &[(&str, &str)]) -> String {
        living_format(self.locale, &format!("living.field.{key}"), params)
    }
    pub(crate) fn field_page_name(&self, page: Page) -> &str {
        self.field_text(&format!("page.{}", page.slug()))
    }
    pub(crate) fn landmark_name(&self, l: Landmark) -> &str {
        self.field_text(&format!("site.{}", l.slug()))
    }
    fn field_near(&self, p: Position) -> bool {
        let own = self.world.player().position;
        self.world.player_sees(p) && own.x.abs_diff(p.x) + own.y.abs_diff(p.y) <= 1
    }
    fn entry(&self, key: &str, command: impl Into<String>, read: bool) -> Entry {
        Entry {
            label: self.field_text(key).into(),
            command: command.into(),
            read,
        }
    }
    fn actor_entry(&self, key: &str, command: &str, a: ActorId, read: bool) -> Entry {
        Entry {
            label: self.field_fmt(key, &[("actor", actor_name(self.locale, &self.world, a))]),
            command: format!("{command} {}", a.get()),
            read,
        }
    }
    pub(crate) fn field_entries(&self, page: Page) -> Vec<Entry> {
        let w = &self.world;
        let p = w.player();
        let region = w.region().expect("charted sandbox");
        let near = |l: Landmark| self.field_near(l.position(w));
        let mut entries = Vec::new();
        macro_rules! add {
            ($key:expr,$cmd:expr,$read:expr) => {
                entries.push(self.entry($key, $cmd, $read));
            };
        }
        match page {
            Page::Local => {
                if !p.alive() {
                    add!("action.wait", "wait", false);
                    return entries;
                }
                if region.background.is_none() {
                    add!("action.wanderer", "background wanderer", false);
                    add!("action.herbalist", "background herbalist", false);
                    add!("action.peddler", "background peddler", false);
                    return entries;
                }
                if near(Landmark::Patch) || near(Landmark::Garden) || near(Landmark::Grove) {
                    add!("action.gather", "gather", false);
                }
                if near(Landmark::Shrine) && p.herbs > 0 {
                    add!("action.deliver", "deliver", false);
                }
                if near(Landmark::Shrine) && p.herbs > 0 && w.breath_progress() < BREATH_MASTERY {
                    add!("action.practice", "practice", false);
                }

                if (near(Landmark::Market) || near(Landmark::Depot)) && !w.is_night() {
                    add!("action.buy", "buy", false);
                    if p.herbs > 0 {
                        add!("action.sell", "sell", false);
                    }
                }
                if near(Landmark::Hall) {
                    if !region.enrolled && w.breath_progress() > 0 && p.herbs > 0 {
                        add!("action.join", "join", false);
                    }
                    if region.enrolled {
                        add!("action.duty", "duty", false);
                        if p.herbs >= 2 {
                            add!("action.brew", "brew", false);
                        }
                        if p.herbs > 0 {
                            add!("action.sponsor", "sponsor", false);
                        }
                    }
                    add!("action.renounce", "renounce", false);
                }
                for n in w
                    .npcs()
                    .filter(|n| n.alive() && self.field_near(n.position))
                {
                    let a = n.id();
                    entries.push(self.actor_entry("action.talk", "talk", a, false));
                    entries.push(self.actor_entry("action.inspect", "inspect", a, true));
                    entries.push(self.actor_entry("action.ask", "ask", a, false));
                    entries.push(self.actor_entry("action.invite", "invite", a, false));
                    if p.herbs > 0 {
                        entries.push(self.actor_entry("action.pack", "pack", a, false));
                    }
                    if w.company()
                        .and_then(|c| c.journey)
                        .is_some_and(|j| j.actor == a)
                    {
                        entries.push(self.actor_entry("action.part", "part", a, false));
                    }
                    entries.push(self.actor_entry("action.aid", "requestaid", a, false));
                    if w.known_bonds().iter().any(|&(id, net)| id == a && net < 0) {
                        entries.push(self.actor_entry("action.repay", "repayaid", a, false));
                    }
                    if w.known_bonds().iter().any(|&(id, net)| id == a && net > 0) {
                        entries.push(self.actor_entry("action.waive", "waiveaid", a, false));
                    }
                    if a == ActorId::new(35) && p.herbs > 0 {
                        add!("action.accord", "accord", false);
                        add!("action.pledge", "pledge", false);
                        add!("action.aidlodge", "aidlodge", false);
                    }
                    if let Some(info) = w.information().and_then(|i| i.known.get(&PLAYER)) {
                        for (&event, _) in info.iter().take(12) {
                            entries.push(Entry {
                                label: self.field_fmt(
                                    "action.tell",
                                    &[
                                        ("actor", actor_name(self.locale, w, a)),
                                        ("id", &event.0.to_string()),
                                    ],
                                ),
                                command: format!("tell {} {}", a.get(), event.0),
                                read: false,
                            });
                        }
                    }
                }
                for cache in w.local_caches() {
                    let id = cache.deposit.id;
                    if cache.can_take {
                        entries.push(Entry {
                            label: self.field_fmt("action.cache.take", &[("id", &id.to_string())]),
                            command: format!("recovercache {id}"),
                            read: false,
                        });
                    }
                    if cache.can_entrust {
                        entries.push(Entry {
                            label: self
                                .field_fmt("action.cache.entrust", &[("id", &id.to_string())]),
                            command: format!("entrustcache {id}"),
                            read: false,
                        });
                    }
                }
                add!("action.wait", "wait", false);
            }
            Page::Travel => {
                add!("action.atlas", "map", true);
                add!("action.survey", "survey", true);
                for l in w.known_landmarks() {
                    entries.push(Entry {
                        label: self.field_fmt("action.route", &[("site", self.landmark_name(l))]),
                        command: format!("route {}", l.slug()),
                        read: true,
                    });
                }
                if self.field.waypoint.is_some() {
                    add!("action.follow", "go", false);
                    add!("action.clear", "route clear", true);
                }
            }
            Page::Training => {
                add!("view.sect", "sect", true);
                add!("view.martial", "martial", true);
                add!("view.meridians", "meridians", true);
                add!("view.cultivation", "cultivation", true);
                add!("view.agency", "agency", true);
                if p.alive() {
                    if near(Landmark::Shrine) && p.herbs > 0 && w.breath_progress() < BREATH_MASTERY
                    {
                        add!("action.practice", "practice", false);
                    }
                    if near(Landmark::Shrine) && region.enrolled {
                        add!("action.deliver", "deliver", false);
                    }
                    if near(Landmark::Hall) && region.enrolled {
                        add!("action.study", "study", false);
                        add!("action.allocate", "allocate", false);
                    }
                    add!("action.drawqi", "drawqi", false);
                    add!("action.circulate", "circulate", false);
                    add!("action.mend", "mend", false);
                    add!("action.center", "center", false);
                    if near(Landmark::Clinic) {
                        add!("action.treat", "treatwounds", false);
                    }
                    if region.remedies > 0 {
                        add!("action.remedy", "remedy", false);
                    }
                    add!("action.guard", "guard", false);
                    for n in w
                        .npcs()
                        .filter(|n| n.alive() && self.field_near(n.position))
                    {
                        let a = n.id();
                        entries.push(self.actor_entry("action.drill", "drill", a, false));
                        entries.push(self.actor_entry("action.attack", "attack", a, false));
                        entries.push(self.actor_entry("action.form", "form", a, false));
                        entries.push(self.actor_entry("action.channel", "channel", a, false));
                        entries.push(self.actor_entry("action.technique", "technique", a, false));
                    }
                    if w.npcs().any(|n| {
                        n.alive() && n.id() == ActorId::new(35) && self.field_near(n.position)
                    }) {
                        add!("action.accord", "accord", false);
                        add!("action.pledge", "pledge", false);
                        add!("action.aidlodge", "aidlodge", false);
                    }
                }
            }
            Page::Commitments => {
                add!("view.quests", "quests", true);
                add!("view.favors", "favors 12", true);
                if p.alive() {
                    for s in w.known_requests() {
                        let r = s.request;
                        let landmark = match r.site {
                            Site::Shrine => Landmark::Shrine,
                            Site::Market => Landmark::Market,
                        };
                        let site = self.landmark_name(landmark);
                        if r.status.active() && w.known_landmarks().any(|l| l == landmark) {
                            entries.push(Entry {
                                label: self.field_fmt(
                                    "action.quest.route",
                                    &[("site", site), ("id", &r.id.to_string())],
                                ),
                                command: format!("route {}", landmark.slug()),
                                read: true,
                            });
                        }
                        let mut action = |key: &str, cmd: &str| {
                            entries.push(Entry {
                                label: self
                                    .field_fmt(key, &[("site", site), ("id", &r.id.to_string())]),
                                command: format!("{cmd} {}", r.id),
                                read: false,
                            })
                        };
                        if r.status == RequestStatus::Offered && w.at_request_site(r.site) {
                            action("action.quest.accept", "acceptquest");
                            action("action.quest.decline", "declinequest");
                        } else if r.status == RequestStatus::Accepted {
                            action("action.quest.abandon", "abandonquest");
                        }
                    }
                    for s in w.known_favors() {
                        let f = s.favor;
                        let local = w.at_favor_actor(f.actor);
                        let mut action = |key: &str, cmd: &str| {
                            entries.push(Entry {
                                label: self.field_fmt(
                                    key,
                                    &[
                                        ("actor", actor_name(self.locale, w, f.actor)),
                                        ("id", &f.id.to_string()),
                                    ],
                                ),
                                command: format!("{cmd} {}", f.id),
                                read: false,
                            })
                        };
                        match f.status {
                            FavorStatus::Offered if local => {
                                action("action.favor.accept", "acceptfavor");
                                action("action.favor.decline", "declinefavor");
                            }
                            FavorStatus::Accepted => {
                                if local && f.need == FavorNeed::CareFunds {
                                    action("action.favor.fund", "fundfavor");
                                }
                                action("action.favor.withdraw", "withdrawfavor");
                            }
                            _ => {}
                        }
                    }
                }
                add!("view.questlog", "questlog 32", true);
                add!("view.favorlog", "favorlog 48", true);
            }
            Page::Journal => {
                for (key, cmd) in [
                    ("governance", "governance"),
                    ("diplomacy", "diplomacy"),
                    ("circles", "circles"),
                    ("profiles", "profiles"),
                    ("stories", "stories"),
                    ("rumors", "rumors"),
                    ("messages", "messages 48"),
                    ("obligations", "obligations 48"),
                    ("teams", "teams 48"),
                    ("social", "sociallog 48"),
                    ("contacts", "contacts 48"),
                    ("bonds", "bonds 48"),
                    ("aid", "aidlog 48"),
                    ("news", "news"),
                    ("production", "production"),
                    ("history", "history"),
                    ("encounters", "encounters"),
                    ("knowledge", "know"),
                    ("contracts", "contractlog"),
                    ("audits", "accountlog"),
                    ("politics", "politicslog"),
                    ("factions", "factionlog"),
                    ("travel", "journal"),
                ] {
                    add!(&format!("view.{key}"), cmd, true);
                }
            }
            Page::System => {
                add!("action.save", "save", true);
                add!("action.load", "load", true);
                add!("view.help", "help", true);
            }
        }
        entries
    }
    pub(crate) fn route_line(&self) -> String {
        let Some(l) = self.field.waypoint else {
            return self.field_text("route.none").into();
        };
        let site = self.landmark_name(l);
        match self.world.charted_route(l) {
            None => self.field_fmt("route.unavailable", &[("site", site)]),
            Some(path) if path.is_empty() => self.field_fmt("route.arrived", &[("site", site)]),
            Some(path) => {
                let d = self.field_text(match path[0] {
                    Direction::North => "direction.north",
                    Direction::East => "direction.east",
                    Direction::South => "direction.south",
                    Direction::West => "direction.west",
                });
                self.field_fmt(
                    "route.next",
                    &[
                        ("site", site),
                        ("steps", &path.len().to_string()),
                        ("direction", d),
                    ],
                )
            }
        }
    }
    pub(crate) fn atlas_lines(&self) -> Vec<String> {
        let w = &self.world;
        let s = w.scenario();
        let p = w.player().position;
        let mut rows = vec![self.field_fmt(
            "atlas.count",
            &[
                ("count", &w.exploration().unwrap().count().to_string()),
                ("total", &(s.width * s.height).to_string()),
            ],
        )];
        for y in 0..s.height {
            let line = (0..s.width)
                .map(|x| {
                    let cell = Position { x, y };
                    if cell == p {
                        '@'
                    } else {
                        w.charted_symbol(cell).unwrap_or('?')
                    }
                })
                .collect::<String>();
            rows.push(line);
        }
        rows.push(self.field_text("atlas.legend").into());
        rows.push(self.route_line());
        rows
    }
    pub(crate) fn field_menu_lines(&self, page: Page) -> Vec<String> {
        let mut lines = vec![
            self.field_page_name(page).to_owned(),
            self.field_text("menu.ascii").into(),
        ];
        lines.extend(
            self.field_entries(page)
                .iter()
                .enumerate()
                .map(|(i, e)| format!("{}. {} [{}]", i + 1, e.label, e.command)),
        );
        lines
    }
    /// Re-derive entries from the current known world; non-field commands
    /// invalidate the open menu. No numeric index outlives its snapshot.
    pub(crate) fn field_input(&mut self, line: &str) -> Option<bool> {
        let lower = line.trim().to_ascii_lowercase();
        let args: Vec<_> = lower.split_whitespace().collect();
        let cmd = args.first().copied().unwrap_or("look");
        let arg = args.get(1).copied();
        if matches!(cmd, "field" | "choose" | "route" | "map" | "go" | "close")
            && (args.len() > 2 || args.len() > 1 && matches!(cmd, "map" | "go" | "close"))
        {
            self.notice = self.field_text("menu.invalid").into();
            return Some(true);
        }
        match cmd {
            "field" => {
                let Some(page) = Page::ALL
                    .into_iter()
                    .find(|p| p.slug() == arg.unwrap_or("local"))
                else {
                    self.notice = self.field_text("menu.invalid").into();
                    return Some(true);
                };
                self.field.open(page);
                self.notice = self.field_menu_lines(page).join("\n");
                Some(true)
            }
            "choose" => {
                let Some(entry) = self.field.page.and_then(|page| {
                    arg.and_then(|a| a.parse::<usize>().ok())
                        .and_then(|i| i.checked_sub(1))
                        .and_then(|i| self.field_entries(page).get(i).cloned())
                }) else {
                    self.notice = self.field_text("menu.selection").into();
                    return Some(true);
                };
                self.field.close();
                let result = self.input(&entry.command);
                if entry.read && !self.field.modal() && !self.notice.is_empty() {
                    self.field.reader = Some(0);
                }
                Some(result)
            }
            "route" => {
                if arg == Some("clear") {
                    self.field.waypoint = None;
                    self.field.close();
                    self.notice = self.route_line();
                    return Some(true);
                }
                if let Some(arg) = arg {
                    let Some(l) = self.world.known_landmarks().find(|l| l.slug() == arg) else {
                        self.notice = self.field_text("route.unknown").into();
                        return Some(true);
                    };
                    self.field.waypoint = Some(l);
                    self.field.close();
                }
                self.notice = self.route_line();
                Some(true)
            }
            "go" => {
                self.field.close();
                let step = self
                    .field
                    .waypoint
                    .and_then(|l| self.world.charted_route(l))
                    .and_then(|r| r.first().copied());
                let Some(d) = step else {
                    self.notice = self.route_line();
                    return Some(true);
                };
                Some(self.input(match d {
                    Direction::North => "north",
                    Direction::East => "east",
                    Direction::South => "south",
                    Direction::West => "west",
                }))
            }
            "map" => {
                self.field.close();
                self.field.atlas = true;
                self.notice = self.atlas_lines().join("\n");
                Some(true)
            }
            "close" => {
                self.field.close();
                Some(true)
            }
            _ => {
                self.field.close();
                if matches!(cmd, "load" | "o" | "reset" | "r" | "seed") {
                    self.field.waypoint = None;
                }
                None
            }
        }
    }
}

/// Wrap without byte slicing. Split even a single long token. Preserve
/// explicit empty lines, as reports may use them to separate paragraphs.
#[cfg(any(feature = "desktop", test))]
pub(crate) fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    assert!(width > 0);
    let mut result = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        for word in raw.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                result.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            for c in word.chars() {
                if line.chars().count() == width {
                    result.push(std::mem::take(&mut line));
                }
                line.push(c);
            }
        }
        if !line.is_empty() || raw.is_empty() {
            result.push(line);
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::living_ascii::sandbox_session;
    #[test]
    fn menu_is_read_only_and_unknown_landmarks_cannot_be_selected() {
        let mut s = sandbox_session(3, false);
        let old = s.world.checkpoint();
        s.input("field travel");
        let labels = s.notice.clone();
        assert!(!labels.contains("route depot"));
        assert!(!labels.contains("route hall"));
        s.input("route depot");
        assert_eq!(s.notice, s.field_text("route.unknown"));
        assert_eq!(s.world.checkpoint(), old);
        s.input("map");
        assert!(s.field.atlas);
        assert!(!s.notice.contains("Stonebank"));
        s.input("route shrine");
        assert!(s.field.waypoint.is_some());
        assert_eq!(s.world.checkpoint(), old);
        s.input("go");
        assert_eq!(s.world.tick(), 1);
        assert!(!s.field.modal());
    }
    #[test]
    fn every_journal_entry_is_a_non_mutating_view() {
        let mut s = sandbox_session(3, false);
        s.input("background herbalist");
        let old = s.world.checkpoint();
        for lang in ["en", "id"] {
            s.input(&format!("lang {lang}"));
            let entries = s.field_entries(Page::Journal);
            assert!(entries.len() >= 20);
            for entry in entries {
                assert!(entry.read);
                s.input(&entry.command);
                assert_eq!(s.world.checkpoint(), old, "{}", entry.command);
                assert!(!s.notice.contains("living.field."));
            }
        }
    }
    #[test]
    fn menu_selection_invalidated_on_turn_and_wrap_is_unicode_safe() {
        let mut s = sandbox_session(0, false);
        s.input("field local");
        s.input("wait");
        let old = s.world.checkpoint();
        s.input("choose 2");
        assert_eq!(old, s.world.checkpoint());
        assert_eq!(
            wrap_lines("abc defghij\n\n\u{00e9}\u{3042}\u{00e9}\u{3042}", 3),
            vec![
                "abc",
                "def",
                "ghi",
                "j",
                "",
                "\u{00e9}\u{3042}\u{00e9}",
                "\u{3042}"
            ]
        );
    }
}
