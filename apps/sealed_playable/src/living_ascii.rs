//! The C1.0 terminal client stamps one canonical turn per gameplay input.
use std::{
    fmt::Write as _,
    fs::File,
    io::{self, BufRead, Write as _},
    path::{Path, PathBuf},
};

use sealed_sim::living::{
    ActorId, AgencyEventKind, Allegiance, AuditFinding, BREATH_MASTERY, Background, CompanyEvent,
    ContractEventKind, DecisionReason, Direction, DisputePhase, Duty, FORM_QI_COST,
    FactionEventKind, Goal, HallRank, HierarchyEventKind, Holder, InfoLogKind, Intent, Invitation,
    JourneyEvent, KnowledgeSource, LoadError, LotEventKind, MAX_QI, MAX_SAVE_BYTES, Need, Outcome,
    PLAYER, PoliticalEventKind, Position, Provenance, Region, Rejection, ReliefEventKind, Role,
    SectDuty, SectEventKind, StepError, TrafficEventKind, World,
};

use crate::{clock, locale::Locale, persistence};

pub(crate) struct Session {
    pub(crate) world: World,
    save_path: PathBuf,
    pub(crate) notice: String,
    pub(crate) locale: Locale,
    debug_world: bool,
}

impl Session {
    pub(crate) fn new(save_path: PathBuf) -> Self {
        Self {
            world: World::information_disputes(0),
            save_path,
            notice: String::new(),
            locale: Locale::default(),
            debug_world: false,
        }
    }

    fn new_seeded_world(&self, seed: u32) -> World {
        if self.world.checkpoint_revision() == 26 {
            World::duty_coherence_campaign(seed)
        } else if self.world.sect_hierarchy().is_some() {
            World::sect_hierarchy_campaign(seed)
        } else if self.world.character_agency().is_some() {
            World::autonomous_agency(seed)
        } else if self.world.injuries().is_some() {
            World::persistent_injury(seed)
        } else if self.world.meridians().is_some() {
            World::meridian_cultivation(seed)
        } else if self.world.martial().is_some() {
            World::martial_mentorship(seed)
        } else if self.world.is_sect_cultivation() {
            World::sect_cultivation(seed)
        } else if self.world.contested().is_some() {
            World::contested_resources(seed)
        } else if self.world.is_physical_recovery() {
            World::physical_recovery(seed)
        } else if self.world.is_player_reporting() {
            World::player_reporting(seed)
        } else if self.world.is_violence_logistics() {
            World::violence_logistics(seed)
        } else if self.world.is_consequences() {
            World::consequences(seed)
        } else if self.world.is_player_agency() {
            World::player_agency(seed)
        } else if self.world.information().is_some() {
            World::information_disputes(seed)
        } else if self.world.contract_memory().is_some() {
            World::contract_accountability(seed)
        } else if self.world.contracts().is_some() {
            World::institutional_agency(seed)
        } else if self.world.faction_agency_state().is_some() {
            World::faction_agency(seed)
        } else if self.world.relief().is_some() {
            World::living_agency(seed)
        } else if self.world.traffic().is_some() {
            World::traffic_courtesy(seed)
        } else if self.world.politics().is_some() {
            World::sect_politics(seed)
        } else if self.world.rivalry().is_some() {
            World::sect_rivalry(seed)
        } else {
            World::road_company(seed)
        }
    }

    /// Returns false on quit. Local inspection and file operations consume no time.
    pub(crate) fn input(&mut self, line: &str) -> bool {
        self.notice.clear();
        let lower = line.trim().to_ascii_lowercase();
        let mut words = lower.split_whitespace();
        let command = words.next().unwrap_or("look");
        let argument = words.next();
        let second = words.next();
        if words.next().is_some()
            || (second.is_some() && !matches!(command, "tell" | "reportbody" | "offerlot"))
            || (argument.is_some()
                && second.is_none()
                && !matches!(
                    command,
                    "invite"
                        | "pack"
                        | "part"
                        | "f"
                        | "attack"
                        | "form"
                        | "e"
                        | "talk"
                        | "i"
                        | "inspect"
                        | "lang"
                        | "seed"
                        | "background"
                        | "ask"
                        | "take"
                        | "claim"
                        | "attune"
                        | "channel"
                        | "drill"
                        | "technique"
                        | "agency"
                ))
        {
            self.notice = text(
                self.locale,
                "Use one command at a time.",
                "Masukkan satu perintah.",
            )
            .into();
            return true;
        }
        let intent = match command {
            "q" | "quit" => return false,
            "w" | "north" => Intent::Move(Direction::North),
            "d" | "east" => Intent::Move(Direction::East),
            "s" | "south" => Intent::Move(Direction::South),
            "a" | "west" => Intent::Move(Direction::West),
            "n" | "step" | "wait" => Intent::Wait,
            "b" | "guard" => Intent::Guard,
            "g" | "gather" => Intent::Gather,
            "h" | "deliver" => Intent::Deliver,
            "m" | "practice" => Intent::Practice,
            "buy" => Intent::BuyHerb,
            "sell" => Intent::SellHerb,
            "join" => Intent::JoinHall,
            "brew" => Intent::Brew,
            "remedy" => Intent::UseRemedy,
            "duty" => Intent::HallDuty,
            "study" => Intent::StudyForm,
            "center" => Intent::CenterQi,
            "drawqi" if self.world.meridians().is_some() => Intent::DrawQi,
            "circulate" if self.world.meridians().is_some() => Intent::CirculateQi,
            "mend" if self.world.meridians().is_some() => Intent::MendMeridians,
            "treatwounds" if self.world.injuries().is_some() => Intent::TreatWounds,
            "allocate" if self.world.sect_hierarchy().is_some() => Intent::ClaimHallAllocation,
            "allocate" => {
                self.notice = text(
                    self.locale,
                    "Hall allocations require a new C3.5 sect hierarchy campaign.",
                    "Alokasi balai memerlukan kampanye hierarki sekte C3.5 baru.",
                )
                .into();
                return true;
            }
            "sect" if self.world.sect_hierarchy().is_some() => {
                let hierarchy = self.world.sect_hierarchy().unwrap();
                let mut parts = Vec::new();
                if let Some(player) = hierarchy.player {
                    let duty = match player.duty {
                        SectDuty::Unassigned => text(self.locale, "none", "tidak ada"),
                        SectDuty::Carrying => {
                            text(self.locale, "carrying offering", "membawa titipan")
                        }
                        SectDuty::Delivered => {
                            text(self.locale, "awaiting report", "menunggu laporan")
                        }
                    };
                    let allocation =
                        if !matches!(player.rank, HallRank::SeniorDisciple | HallRank::Elder) {
                            text(
                                self.locale,
                                "requires senior disciple",
                                "butuh murid senior",
                            )
                        } else if player.allocation_claimed {
                            text(self.locale, "claimed", "sudah diambil")
                        } else {
                            text(self.locale, "available", "tersedia")
                        };
                    parts.push(format!(
                        "{}: {} ({} {}); {}: {duty}; {}: {allocation}",
                        text(self.locale, "Reed Hall", "Balai Alang-alang"),
                        hall_rank_label(self.locale, player.rank),
                        player.merit,
                        text(self.locale, "merit", "jasa"),
                        text(self.locale, "duty", "tugas"),
                        text(self.locale, "allocation", "alokasi"),
                    ));
                } else {
                    parts.push(
                        text(
                            self.locale,
                            "Not enrolled in Reed Hall.",
                            "Belum terdaftar di Balai Alang-alang.",
                        )
                        .into(),
                    );
                }
                for (&id, &member) in &hierarchy.members {
                    if let Some(npc) = self.world.npc(id)
                        && npc.alive()
                        && self.world.player_sees(npc.position)
                    {
                        parts.push(format!(
                            "{}: {}",
                            npc.definition.name,
                            hall_rank_label(self.locale, member.rank)
                        ));
                    }
                }
                self.notice = parts.join(" | ");
                return true;
            }
            "sectlog" if self.world.sect_hierarchy().is_some() => {
                let lines: Vec<_> = self
                    .world
                    .sect_hierarchy()
                    .unwrap()
                    .history
                    .iter()
                    .rev()
                    .filter(|event| event.witnessed)
                    .take(12)
                    .map(|event| {
                        let actor = actor_name(self.locale, &self.world, event.actor);
                        let rank = hall_rank_label(self.locale, event.rank);
                        match event.kind {
                            HierarchyEventKind::DutyAccepted => format!(
                                "#{} t{} {actor} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "accepted a Reed Hall duty",
                                    "menerima tugas Balai Alang-alang"
                                )
                            ),
                            HierarchyEventKind::DutyDelivered => format!(
                                "#{} t{} {actor} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "delivered a Hall offering to the shrine",
                                    "mengantar persembahan balai ke kuil"
                                )
                            ),
                            HierarchyEventKind::MeritEarned => format!(
                                "#{} t{} {actor} {} ({})",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "earned Reed Hall merit",
                                    "meraih jasa Balai Alang-alang"
                                ),
                                event.merit
                            ),
                            HierarchyEventKind::Promoted => format!(
                                "#{} t{} {actor} {} {rank}",
                                event.id,
                                event.at,
                                text(self.locale, "promoted to", "naik pangkat menjadi")
                            ),
                            HierarchyEventKind::AllocationClaimed => format!(
                                "#{} t{} {actor} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "claimed a physical Hall herb allocation",
                                    "mengambil alokasi herba fisik balai"
                                )
                            ),
                            HierarchyEventKind::OfficeAssumed => format!(
                                "#{} t{} {actor} {} {rank}",
                                event.id,
                                event.at,
                                text(self.locale, "assumed office as", "menjabat sebagai")
                            ),
                        }
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed sect hierarchy events.",
                        "Belum ada peristiwa hierarki sekte yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "wounds" if self.world.injuries().is_some() => {
                let state = self.world.injuries().unwrap().people[&PLAYER];
                self.notice = format!("Arm wounds: {}; leg wounds: {}", state.arms, state.legs);
                return true;
            }
            "woundlog" if self.world.injuries().is_some() => {
                let lines: Vec<_> = self
                    .world
                    .injuries()
                    .unwrap()
                    .events
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        format!(
                            "#{} t{} {} {:?}",
                            event.id,
                            event.at,
                            actor_name(self.locale, &self.world, event.actor),
                            event.kind
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    "No witnessed wounds.".into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "agency" if self.world.character_agency().is_some() => {
                let Some(id) = argument.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice = "agency <visible npc>".into();
                    return true;
                };
                let id = ActorId::new(id);
                let Some(npc) = self
                    .world
                    .npc(id)
                    .filter(|npc| npc.alive() && self.world.player_sees(npc.position))
                else {
                    self.notice = text(
                        self.locale,
                        "That character is not visible.",
                        "Tokoh itu tidak terlihat.",
                    )
                    .into();
                    return true;
                };
                let observed = self
                    .world
                    .character_agency()
                    .unwrap()
                    .history
                    .iter()
                    .rev()
                    .find(|event| {
                        event.actor == id
                            && event.witnessed
                            && event.kind == AgencyEventKind::GoalChanged
                    });
                self.notice = observed.map_or_else(
                    || {
                        format!(
                            "{}: {}",
                            npc.definition.name,
                            text(
                                self.locale,
                                "you have not witnessed a change of purpose",
                                "kamu belum menyaksikan perubahan tujuannya",
                            )
                        )
                    },
                    |event| {
                        format!(
                            "{}: {} {} — {}",
                            npc.definition.name,
                            text(self.locale, "you observed", "kamu menyaksikan"),
                            event
                                .goal
                                .map(|goal| goal_label(self.locale, goal))
                                .unwrap_or("?"),
                            event
                                .reason
                                .map(|reason| decision_reason_label(self.locale, reason))
                                .unwrap_or("?")
                        )
                    },
                );
                return true;
            }
            "agencylog" if self.world.character_agency().is_some() => {
                let lines: Vec<_> = self
                    .world
                    .character_agency()
                    .unwrap()
                    .history
                    .iter()
                    .rev()
                    .filter(|event| event.witnessed)
                    .take(12)
                    .map(|event| {
                        let actor = actor_name(self.locale, &self.world, event.actor);
                        match event.kind {
                            AgencyEventKind::GoalChanged => format!(
                                "#{} t{} {actor}: {} ({})",
                                event.id,
                                event.at,
                                event
                                    .goal
                                    .map(|goal| goal_label(self.locale, goal))
                                    .unwrap_or("?"),
                                event
                                    .reason
                                    .map(|reason| decision_reason_label(self.locale, reason))
                                    .unwrap_or("?")
                            ),
                            AgencyEventKind::WorkGathered => format!(
                                "#{} t{} {actor} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "gathered work herbs",
                                    "memetik herba untuk bekerja",
                                )
                            ),
                            AgencyEventKind::WorkSold => format!(
                                "#{} t{} {actor} {} {} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "sold work herbs for",
                                    "menjual herba kerja senilai",
                                ),
                                event.amount,
                                text(self.locale, "coin(s)", "koin")
                            ),
                            AgencyEventKind::CarePaid => format!(
                                "#{} t{} {actor} {} {} {}",
                                event.id,
                                event.at,
                                text(self.locale, "paid", "membayar",),
                                event.amount,
                                text(
                                    self.locale,
                                    "coin(s) for wound care",
                                    "koin untuk perawatan luka",
                                )
                            ),
                            AgencyEventKind::CareBartered => format!(
                                "#{} t{} {actor} {}",
                                event.id,
                                event.at,
                                text(
                                    self.locale,
                                    "bartered an herb for wound care",
                                    "menukar satu herba untuk perawatan luka",
                                )
                            ),
                        }
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "You have witnessed no character decisions.",
                        "Kamu belum menyaksikan keputusan tokoh.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "meridians" if self.world.meridians().is_some() => {
                let state = self.world.meridians().unwrap().people[&PLAYER];
                self.notice = format!(
                    "Qi: {}; open channels: {}; blockage: {}",
                    state.qi, state.channels, state.blockage
                );
                return true;
            }
            "meridianlog" if self.world.meridians().is_some() => {
                let lines: Vec<_> = self
                    .world
                    .meridians()
                    .unwrap()
                    .events
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        format!(
                            "#{} t{} {} {:?}",
                            event.id,
                            event.at,
                            actor_name(self.locale, &self.world, event.actor),
                            event.kind
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    "No witnessed meridian events.".into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "accord" => Intent::OfferAccord,
            "sponsor" => Intent::SupportHall,
            "pledge" => Intent::PledgeLodge,
            "aidlodge" => Intent::AidLodge,
            "renounce" => Intent::RenounceLodge,
            "examine" => Intent::Examine,
            "ask" => match self.target(argument) {
                Ok(id) => Intent::Ask(id),
                Err(message) => {
                    self.notice = message.into();
                    return true;
                }
            },
            "take" => match self.body(argument) {
                Ok(id) => Intent::Take(id),
                Err(message) => {
                    self.notice = message.into();
                    return true;
                }
            },
            "tell" => {
                let Some(actor) = argument.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice =
                        text(self.locale, "tell <npc> <event>", "tell <npc> <peristiwa>").into();
                    return true;
                };
                let Some(event) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice =
                        text(self.locale, "tell <npc> <event>", "tell <npc> <peristiwa>").into();
                    return true;
                };
                Intent::Testify {
                    to: ActorId::new(actor),
                    event: sealed_sim::living::EventId(event),
                }
            }
            "reportbody" => {
                if !self.world.is_player_reporting() {
                    self.notice = text(self.locale, "Only in C2.7.", "Hanya di C2.7.").into();
                    return true;
                }
                let Some(body) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice = text(
                        self.locale,
                        "reportbody <npc> <body>",
                        "reportbody <npc> <jenazah>",
                    )
                    .into();
                    return true;
                };
                let to = match self.target(argument) {
                    Ok(id) => id,
                    Err(message) => {
                        self.notice = message.into();
                        return true;
                    }
                };
                Intent::ReportBody {
                    to,
                    body: ActorId::new(body),
                }
            }
            "claim" => {
                let Some(id) = argument.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice = text(self.locale, "claim <lot>", "claim <barang>").into();
                    return true;
                };
                if self.world.contested().is_none() {
                    self.notice = text(self.locale, "Only in C2.9.", "Hanya di C2.9.").into();
                    return true;
                }
                Intent::ClaimLot(id)
            }
            "attune" => {
                let Some(id) = argument.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice = "attune <lot>".into();
                    return true;
                };
                if !self.world.is_sect_cultivation() {
                    self.notice = "Only in C3.0.".into();
                    return true;
                }
                Intent::AttuneLot(id)
            }
            "cultivation" => {
                if !self.world.is_sect_cultivation() {
                    self.notice = "Only in C3.0.".into();
                    return true;
                }
                let state = self.world.contested().unwrap();
                self.notice = format!(
                    "Wards: {}; Jade channels: {}",
                    state.wards, state.script_charges
                );
                return true;
            }
            "martial" => {
                let Some(martial) = self.world.martial() else {
                    self.notice = "Only in C3.1.".into();
                    return true;
                };
                let skill = martial.skills[&PLAYER];
                self.notice = format!(
                    "Form: {:?}; lessons: {}; strain: {}; parry: {}",
                    skill.form, skill.lessons, skill.strain, skill.parry
                );
                return true;
            }
            "martiallog" => {
                let Some(martial) = self.world.martial() else {
                    self.notice = "Only in C3.1.".into();
                    return true;
                };
                let lines: Vec<_> = martial
                    .lessons
                    .iter()
                    .filter(|lesson| lesson.witnessed)
                    .map(|lesson| {
                        format!(
                            "#{} t{} {} trained with {} in {:?}{}",
                            lesson.id,
                            lesson.at,
                            actor_name(self.locale, &self.world, lesson.student),
                            actor_name(self.locale, &self.world, lesson.teacher),
                            lesson.form,
                            if lesson.learned { " (learned)" } else { "" }
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    "No witnessed lessons.".into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "offerlot" => {
                let Some(id) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice = text(
                        self.locale,
                        "offerlot <npc> <lot>",
                        "offerlot <npc> <barang>",
                    )
                    .into();
                    return true;
                };
                if self.world.contested().is_none() {
                    self.notice = text(self.locale, "Only in C2.9.", "Hanya di C2.9.").into();
                    return true;
                }
                let to = match self.target(argument) {
                    Ok(id) => id,
                    Err(message) => {
                        self.notice = message.into();
                        return true;
                    }
                };
                Intent::OfferLot { to, id }
            }
            "lots" => {
                let Some(state) = self.world.contested() else {
                    self.notice = text(self.locale, "Only in C2.9.", "Hanya di C2.9.").into();
                    return true;
                };
                let lines: Vec<_> = state
                    .lots
                    .values()
                    .filter_map(|lot| {
                        let place = match lot.holder {
                            Holder::Ground(position) if self.world.player_sees(position) => {
                                format!("({},{})", position.x, position.y)
                            }
                            Holder::Carried(PLAYER) => {
                                text(self.locale, "your pack", "tasmu").into()
                            }
                            _ => return None,
                        };
                        Some(format!("#{} {:?} {place}", lot.id, lot.good))
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No visible lots or carried relics.",
                        "Tidak ada barang terlihat atau relik yang dibawa.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "lotlog" => {
                let Some(state) = self.world.contested() else {
                    self.notice = text(self.locale, "Only in C2.9.", "Hanya di C2.9.").into();
                    return true;
                };
                let lines: Vec<_> = state
                    .history
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        let verb = match event.kind {
                            LotEventKind::Claimed => text(self.locale, "claimed", "mengambil"),
                            LotEventKind::Sold => text(self.locale, "sold", "menjual"),
                            LotEventKind::Deposited => {
                                text(self.locale, "deposited", "menyerahkan")
                            }
                            LotEventKind::Dropped => text(self.locale, "dropped", "menjatuhkan"),
                            LotEventKind::Attuned => text(self.locale, "attuned", "mengolah"),
                        };
                        format!(
                            "#{} t{} {} {} lot #{}",
                            event.id,
                            event.at,
                            actor_name(self.locale, &self.world, event.actor),
                            verb,
                            event.lot
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed lot transfers.",
                        "Belum ada perpindahan barang yang disaksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "know" => {
                if !self.world.is_player_agency() {
                    self.notice = text(
                        self.locale,
                        "know is part of the C2.3 campaign.",
                        "know ada di kampanye C2.3.",
                    )
                    .into();
                    return true;
                }
                let lines: Vec<String> = self
                    .world
                    .information()
                    .and_then(|info| info.known.get(&PLAYER))
                    .map(|known| {
                        known
                            .iter()
                            .filter_map(|(&id, receipt)| {
                                let event = self.world.information()?.event(id)?;
                                let how = match receipt.provenance {
                                    Provenance::Firsthand { .. } => {
                                        text(self.locale, "witnessed", "disaksikan")
                                    }
                                    Provenance::Reported { sender, .. } if sender == PLAYER => {
                                        text(self.locale, "examined", "diperiksa")
                                    }
                                    Provenance::Reported { hops, .. } => text(
                                        self.locale,
                                        if hops <= 1 { "heard" } else { "rumor" },
                                        if hops <= 1 { "didengar" } else { "desas-desus" },
                                    ),
                                };
                                let finding = match event.finding {
                                    AuditFinding::Delivered => {
                                        text(self.locale, "delivered", "terkirim")
                                    }
                                    AuditFinding::Unfulfilled => {
                                        text(self.locale, "missed", "gagal")
                                    }
                                };
                                Some(format!(
                                    "#{id} {finding} {} ({how})",
                                    actor_name(self.locale, &self.world, event.provider),
                                    id = id.0
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "You know no institutional finding.",
                        "Kamu belum mengetahui temuan lembaga.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "bodies" => {
                if !self.world.is_violence_logistics() {
                    self.notice = text(self.locale, "Only in C2.5.", "Hanya di C2.5.").into();
                    return true;
                }
                let lines: Vec<_> = self
                    .world
                    .social()
                    .discoveries()
                    .filter(|discovery| discovery.observer == PLAYER)
                    .map(|discovery| {
                        format!(
                            "t{} {} ({})",
                            discovery.at,
                            actor_name(self.locale, &self.world, discovery.body),
                            text(
                                self.locale,
                                "cause unknown unless you witnessed it",
                                "penyebab tidak diketahui kecuali kamu menyaksikannya"
                            )
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "You have discovered no bodies.",
                        "Kamu belum menemukan jenazah.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "salvagelog" => {
                if !self.world.is_physical_recovery() {
                    self.notice = text(self.locale, "Only in C2.8.", "Hanya di C2.8.").into();
                    return true;
                }
                let lines: Vec<_> = self
                    .world
                    .social()
                    .recoveries()
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        format!(
                            "#{} t{} {} {} {} {} {}",
                            event.id,
                            event.at,
                            actor_name(self.locale, &self.world, event.actor),
                            text(
                                self.locale,
                                if event.delivered {
                                    "delivered"
                                } else {
                                    "recovered"
                                },
                                if event.delivered {
                                    "mengantar"
                                } else {
                                    "mengambil"
                                }
                            ),
                            event.herbs,
                            text(self.locale, "herbs from", "herba dari"),
                            actor_name(self.locale, &self.world, event.body)
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed cargo recovery.",
                        "Belum menyaksikan pemulihan muatan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "crimelog" => {
                if !self.world.is_violence_logistics() {
                    self.notice = text(self.locale, "Only in C2.5.", "Hanya di C2.5.").into();
                    return true;
                }
                let lines: Vec<_> = self
                    .world
                    .social()
                    .incidents()
                    .iter()
                    .rev()
                    .filter_map(|incident| {
                        let relation = if incident.source == PLAYER {
                            text(self.locale, "you did this", "kamu melakukannya")
                        } else if incident.target == PLAYER {
                            text(self.locale, "you were struck", "kamu diserang")
                        } else {
                            match self.world.social().knowledge(PLAYER, incident.id)?.source {
                                KnowledgeSource::Observed => {
                                    text(self.locale, "witnessed", "disaksikan")
                                }
                                KnowledgeSource::Reported { hops: 1, .. } => {
                                    text(self.locale, "heard firsthand", "dengar langsung")
                                }
                                KnowledgeSource::Reported { .. } => {
                                    text(self.locale, "allegation", "dugaan")
                                }
                            }
                        };
                        let action = if incident.damage == 0 {
                            text(self.locale, "took cargo from", "mengambil muatan dari")
                        } else {
                            text(self.locale, "hit", "menyerang")
                        };
                        Some(format!(
                            "t{} {} {} {} ({relation})",
                            incident.tick,
                            actor_name(self.locale, &self.world, incident.source),
                            action,
                            actor_name(self.locale, &self.world, incident.target)
                        ))
                    })
                    .take(12)
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "You know no assaults or thefts.",
                        "Kamu belum mengetahui serangan atau pencurian.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "background" => {
                let background = match argument {
                    Some("wanderer") => Background::Wanderer,
                    Some("herbalist") => Background::Herbalist,
                    Some("peddler") => Background::Peddler,
                    _ => {
                        self.notice = "background wanderer | herbalist | peddler".into();
                        return true;
                    }
                };
                Intent::ChooseBackground(background)
            }
            "seed" => {
                let seed = argument.and_then(|value| value.parse::<u32>().ok());
                self.notice = match seed {
                    Some(seed) if self.world.tick() == 0 => {
                        self.world = self.new_seeded_world(seed);
                        format!("{} {seed}", text(self.locale, "World seed", "Seed dunia"))
                    }
                    _ => text(
                        self.locale,
                        "Use seed <number> before the first turn.",
                        "Gunakan seed <angka> sebelum giliran pertama.",
                    )
                    .into(),
                };
                return true;
            }
            "help" => {
                self.notice = if self.world.checkpoint_revision() == 26 {
                    text(self.locale,
                        "C3.5.1: Reed Hall members finish active shrine deliveries and reports before joining your company. They cannot accept new Hall delivery duties while accompanying you. sect and sectlog inspect known rank and witnessed events; duty, drill, drawqi, study and allocate retain their C3.5 rules. Use --c35 for the original C3.5 save.",
                        "C3.5.1: Anggota Balai Alang-alang menuntaskan pengantaran dan laporan tugas kuil sebelum bergabung dengan rombonganmu. Mereka tidak menerima tugas pengantaran balai baru saat menemanimu. sect dan sectlog menampilkan pangkat yang diketahui serta peristiwa yang disaksikan; duty, drill, drawqi, study dan allocate tetap mengikuti aturan C3.5. Gunakan --c35 untuk simpanan C3.5 asli.")
                } else if self.world.sect_hierarchy().is_some() {
                    text(self.locale,
                        "C3.5: Reed Hall tracks persisted rank and merit for both you and NPC members. Complete shrine duty to reach Disciple (free Reed Hall martial drills, Reed Strike study, and Hall Qi access), and repeat Hall duty from real Hall offerings to reach Senior Disciple and claim one physical herb with allocate. NPC members perform duties, advance, and assume Elder office on succession without you. sect and sectlog inspect known standing and witnessed events. --c34 keeps C3.4.",
                        "C3.5: Balai Alang-alang mencatat pangkat dan jasa bagi dirimu maupun anggota NPC. Tuntaskan tugas kuil untuk menjadi Murid Dalam (latihan bela diri balai bebas biaya, belajar Jurus Alang-alang, dan akses Qi balai), lalu ulangi tugas dari persembahan nyata balai untuk menjadi Murid Senior dan mengambil satu herba fisik dengan allocate. Anggota NPC menjalankan tugas, naik pangkat, dan menjabat Tetua saat suksesi tanpa bantuanmu. sect dan sectlog memeriksa kedudukan dan peristiwa yang disaksikan. --c34 untuk C3.4.")
                } else if self.world.character_agency().is_some() {
                    text(self.locale,
                        "C3.4: Injured NPCs without treatment money can gather and sell physical herbs, pay a present medic, recover, and resume interrupted ambitions. agency <visible npc> explains current circumstances; agencylog lists only decisions and transactions you witnessed. --c33 keeps free-care C3.3.",
                        "C3.4: NPC terluka tanpa biaya perawatan dapat memetik dan menjual herba nyata, membayar tabib yang hadir, pulih, lalu melanjutkan ambisi yang terputus. agency <npc terlihat> menjelaskan keadaan saat ini; agencylog hanya mencatat keputusan dan transaksi yang kamu saksikan. --c33 mempertahankan C3.3 dengan perawatan gratis.")
                } else if self.world.injuries().is_some() {
                    text(self.locale,
                        "C3.3: Real blows can leave lasting arm or leg wounds. Severe arm wounds weaken strikes; severe leg wounds slow movement on alternating ticks. Injured NPCs retreat and seek care. At a staffed clinic treatwounds spends one shrine herb and heals one wound; wounds and woundlog show your condition and witnessed events. --c32 keeps C3.2.",
                        "C3.3: Pukulan nyata dapat meninggalkan luka lengan atau kaki. Luka lengan berat melemahkan serangan; luka kaki berat memperlambat gerak setiap dua giliran. NPC terluka mundur dan mencari perawatan. Di klinik yang dijaga treatwounds memakai satu herba kuil dan menyembuhkan satu luka; wounds dan woundlog menunjukkan kondisi serta peristiwa yang disaksikan. --c32 untuk C3.2.")
                } else if self.world.meridians().is_some() {
                    text(self.locale,
                        "C3.2: Gather real herbs; at Qinghe shrine or Reed Hall drawqi spends one herb for one Qi. circulate spends Qi: a rested body opens a channel; strain or wounds obstruct it. Two open channels and Qi are required for technique. Blockage requires mend at a staffed clinic, using real shrine medicine. meridians and meridianlog show known condition. --c31 keeps C3.1.",
                        "C3.2: Petik herba sungguhan; di kuil Qinghe atau Balai Alang-alang drawqi memakai satu herba untuk satu Qi. circulate memakai Qi: tubuh pulih membuka jalur, latihan berat atau luka menyumbatnya. Dua jalur terbuka dan Qi diperlukan untuk technique. Sumbatan dirawat dengan mend di klinik yang dijaga, memakai obat kuil. meridians dan meridianlog mencatat kondisi. --c31 untuk C3.1.")
                } else if self.world.martial().is_some() {
                    text(self.locale,
                        "C3.1: Seek Ren Zhi (33) for Reed Palm or Stonebank's envoy (35) for Stone Fist. drill <teacher> twice face-to-face (one coin per lesson); martial and martiallog show personal and witnessed progress. technique <npc> uses your learned form; wait eases strain. Sect oaths restrict rival lessons. NPC disciples train without you. --c30 keeps C3.0.",
                        "C3.1: Temui Ren Zhi (33) untuk Telapak Alang-alang atau utusan Batu Tepi (35) untuk Tinju Batu. drill <guru> dua kali langsung (satu koin per pelajaran); martial dan martiallog menampilkan latihan diri dan yang disaksikan. technique <npc> memakai jurus; wait meredakan kelelahan. Sumpah perguruan membatasi guru saingan. Murid NPC berlatih sendiri. --c30 untuk C3.0.")
                } else if self.world.is_sect_cultivation() {
                    text(self.locale,
                        "C3.0: Keep a relic to cultivate instead of selling. attune <lot> consumes carried SpiritOre into two one-hit wards or JadeScript into two ranged channel <npc> strikes. Use cultivation to inspect charges. --c29 preserves C2.9.",
                        "C3.0: Simpan relik untuk berkultivasi alih-alih menjual. attune <barang> menghabiskan SpiritOre menjadi dua perlindungan atau JadeScript menjadi dua serangan jarak jauh channel <npc>. Gunakan cultivation untuk melihat sisa. --c29 untuk C2.9.")
                } else if self.world.contested().is_some() {
                    text(self.locale,
                        "C2.9: Rival sects compete for finite SpiritOre and JadeScript lots. Use lots, lotlog, claim <lot>, offerlot <npc> <lot> to sell a carried good face-to-face to Stonebank's envoy or Reed Hall's steward. Unseen goods stay private. --c28 preserves the previous campaign.",
                        "C2.9: Perguruan bersaing memperebutkan SpiritOre dan JadeScript yang terbatas. Gunakan lots, lotlog, claim <barang>, offerlot <npc> <barang> untuk menjual barang langsung kepada utusan Stonebank atau pengurus Balai Alang-alang. Barang tak terlihat tetap tersembunyi. --c28 untuk kampanye sebelumnya.")
                } else if self.world.is_physical_recovery() {
                    text(self.locale,
                        "C2.8: Watchmen who see a body can recover its real herbs and carry them back to the shrine when medicine is short. The body is not evidence of a killer. Use reportbody <npc> <body>, bodies and salvagelog. --c27 keeps C2.7; --c25 keeps C2.5.",
                        "C2.8: Penjaga yang melihat jenazah dapat mengambil herba sungguhan dan membawanya ke kuil saat obat kurang. Jenazah bukan bukti pembunuh. Gunakan reportbody <npc> <jenazah>, bodies dan salvagelog. --c27 untuk C2.7; --c25 untuk C2.5.")
                } else if self.world.is_player_reporting() {
                    text(self.locale,
                        "C2.7: Find a body, then reportbody <npc> <body> face-to-face. The listener hears a corpse report after two turns, not a murder accusation. A watchman may travel to confirm it. bodies and crimelog show what you personally know. --c25 keeps C2.5; --c24 keeps C2.4.",
                        "C2.7: Temukan jenazah, lalu reportbody <npc> <jenazah> secara tatap muka. Penerima mendengar laporan jenazah dua giliran kemudian, bukan tuduhan pembunuhan. Penjaga dapat pergi memeriksanya. bodies dan crimelog mencatat yang kamu ketahui. --c25 untuk C2.5; --c24 untuk C2.4.")
                } else if self.world.is_violence_logistics() {
                    text(self.locale,
                        "C2.5: attack <id> a living courier or contract provider; carried herbs remain on the body. take <id> recovers them. bodies shows only corpses you discovered, not the killer. crimelog lists only your own actions or learned incidents. Witnesses identify attackers; nearby strangers only have a lead. contractlog, accountlog, infolog, know track consequences. --c24 keeps C2.4.",
                        "C2.5: attack <id> kurir atau pengantar kontrak; herba tetap di jenazah. take <id> mengambilnya. bodies hanya menunjukkan jenazah yang ditemukan, bukan pembunuhnya. crimelog hanya mencatat tindakanmu atau kejadian yang kamu ketahui. Saksi mengenali penyerang; orang dekat hanya punya petunjuk. contractlog, accountlog, infolog, know mencatat akibat. --c24 untuk C2.4.")
                } else if self.world.is_consequences() {
                    text(
                        self.locale,
                        "C2.4: A dead carrier's herbs can be taken from the body. People who see you do it remember who you are. People who do not only notice that the cargo is gone. examine, ask, tell, know, and take [id]. --c23 keeps C2.3; --c22 keeps C2.2.",
                        "C2.4: Herba pada jenazah pengangkut dapat diambil. Orang yang melihatmu mengingat pelakunya. Orang yang tidak melihat hanya mendapati barangnya hilang. examine, ask, tell, know, dan take [id]. --c23 untuk C2.3; --c22 untuk C2.2.",
                    )
                } else if self.world.is_player_agency() {
                    text(
                        self.locale,
                        "C2.3: You can witness, examine a site, ask someone what they know, or tell a specific finding to someone beside you. examine, ask [id], tell [id] [event], and know. Gossip you only heard does not carry the same weight as evidence you checked. --c22 keeps C2.2; --c21 keeps C2.1; --c20 keeps C2.0.",
                        "C2.3: Kamu dapat menyaksikan, memeriksa lokasi, bertanya apa yang diketahui seseorang, atau menyampaikan temuan tertentu kepada orang di sampingmu. examine, ask [id], tell [id] [event], dan know. Gosip yang hanya kamu dengar tidak sekuat bukti yang kamu periksa. --c22 untuk C2.2; --c21 untuk C2.1; --c20 untuk C2.0.",
                    )
                } else if self.world.information().is_some() {
                    text(
                        self.locale,
                        "C2.2: Audit findings travel only by face-to-face testimony with provenance. Credible reports can open institutional disputes that change later contract awards, including suspension. Use contractlog, accountlog and infolog. --c21 keeps C2.1; --c20 keeps C2.0.",
                        "C2.2: Temuan audit hanya menyebar lewat kesaksian tatap muka dengan asal-usul. Laporan kredibel dapat membuka sengketa lembaga yang mengubah kontrak berikutnya, termasuk penangguhan. Gunakan contractlog, accountlog dan infolog. --c21 untuk C2.1; --c20 untuk C2.0.",
                    )
                } else if self.world.contract_memory().is_some() {
                    text(
                        self.locale,
                        "C2.1: Institutions inspect their own delivery sites before judging past providers. Confirmed deliveries earn local trust; missed deadlines reduce it. Those memories influence later in-person bids, never remote knowledge. Use contractlog and accountlog. --c20 keeps the original contract campaign.",
                        "C2.1: Lembaga harus memeriksa lokasi pengiriman sendiri sebelum menilai penyedia. Pengiriman terkonfirmasi membangun kepercayaan; tenggat terlewat menguranginya. Ingatan ini memengaruhi tawaran tatap muka berikutnya, bukan pengetahuan jarak jauh. Gunakan contractlog dan accountlog. --c20 untuk kampanye kontrak lama.",
                    )
                } else if self.world.contracts().is_some() {
                    text(
                        self.locale,
                        "C2.0: Qinghe and Reed Hall may discover shortages locally. Requesters travel to prospective sponsors, collect offers, return to award the chosen institution, then wait for real market procurement and physical delivery. Contracts can expire or fail. Use contractlog for events you witnessed; --c19 retains the earlier autonomous faction campaign.",
                        "C2.0: Qinghe dan Balai Alang-alang dapat mengenali kekurangan secara lokal. Pemohon mengunjungi calon penyokong, membawa tawaran, kembali menyampaikan keputusan, lalu menunggu pembelian pasar dan pengiriman fisik. Kontrak bisa gagal atau kedaluwarsa. Gunakan contractlog untuk kejadian yang kamu saksikan; --c19 untuk kampanye faksi sebelumnya.",
                    )
                } else if self.world.faction_agency_state().is_some() {
                    text(
                        self.locale,
                        "C1.9: Stonebank's envoy periodically visits Qinghe to inspect the shrine. A shortage witnessed in person can provoke a competing physical aid shipment, paid with Stonebank funds. Both factions contend for finite market herbs. Use aidlog for events you witnessed; --c18 keeps the earlier campaign.",
                        "C1.9: Utusan Batu Tepi mendatangi Qinghe untuk memeriksa kuil. Kekurangan yang disaksikan sendiri dapat memicu pengiriman bantuan fisik memakai uang perguruan. Kedua faksi bersaing atas stok pasar yang terbatas. Gunakan aidlog untuk kejadian yang kamu saksikan; --c18 untuk kampanye lama.",
                    )
                } else if self.world.relief().is_some() {
                    text(
                        self.locale,
                        "C1.8: An observed Qinghe medicine shortage may send the watchman to ask Reed Hall for relief. The steward must obtain real herbs at Willowford market and carry them to the shrine. The mission can fail or be withdrawn. Use relieflog for events you witnessed, or trafficlog for passage.",
                        "C1.8: Kekurangan obat Qinghe yang terlihat dapat membuat penjaga meminta bantuan Balai Alang-alang. Pengurus harus membeli herba nyata di pasar Willowford dan membawanya ke kuil. Misi bisa gagal atau dibatalkan. Gunakan relieflog untuk peristiwa yang kamu saksikan, atau trafficlog untuk pertukaran jalan.",
                    )
                } else if self.world.traffic().is_some() {
                    text(
                        self.locale,
                        "C1.7: Move into an adjacent friendly villager to exchange tiles. NPCs can negotiate bottlenecks after a short wait, but busy healers, teachers and hostile actors will not yield. Use trafficlog to inspect witnessed exchanges. Sect politics and Willowford continue as usual.",
                        "C1.7: Bergerak ke petak warga ramah yang bersebelahan untuk bertukar tempat. NPC dapat melewati jalan sempit setelah menunggu sebentar, tetapi tabib atau guru yang sibuk dan musuh tidak akan mengalah. Gunakan trafficlog untuk melihat pertukaran yang disaksikan. Politik perguruan dan Willowford tetap berjalan.",
                    )
                } else if self.world.politics().is_some() {
                    text(self.locale,
                        "Reed Hall and Stonebank compete over Willowford herbs. Visit the market or Hall and use factions. Talk to envoy 35, then pledge with one herb; aidlodge supplies your allies. Renounce at Reed Hall before joining it again. Sponsoring the Hall earns its steward's trust. Use politicslog for witnessed events.",
                        "Balai Alang-alang dan Batu Tepi berebut herba Willowford. Datangi pasar atau balai lalu gunakan factions. Bicara dengan utusan 35, lalu pledge dengan satu herba; aidlodge membekali sekutumu. Renounce di Balai Alang-alang sebelum kembali bergabung. Sponsor balai membangun kepercayaan pengurusnya. Gunakan politicslog untuk peristiwa yang disaksikan.")
                } else if self.world.region().is_some() {
                    text(self.locale,
                        "Follow the eastern road to Willowford. Gather herbs and sell at the market, donate in Qinghe or train at the shrine. Reed Hall teaches remedy brewing after breath practice and a one-herb offering.",
                        "Ikuti jalan ke timur menuju Willowford. Petik herba lalu jual di pasar, sumbangkan di Qinghe, atau pakai untuk latihan di kuil. Balai Alang-alang mengajar racikan obat setelah latihan napas dan persembahan satu herba.")
                } else {
                    text(self.locale,
                        "Gather herbs at the patch and deliver them to the shrine. Use inspect on visible villagers and talk to the healer for care.",
                        "Petik herba di ladang lalu antar ke kuil. Gunakan inspect untuk warga yang terlihat dan bicara dengan tabib untuk berobat.")
                }.into();
                return true;
            }
            "lang" => {
                match argument {
                    Some("en") => self.locale = Locale::English,
                    Some("id") => self.locale = Locale::Indonesian,
                    _ => self.notice = "lang en | lang id".into(),
                }
                return true;
            }
            "i" | "inspect" => {
                self.notice = self.inspect(argument);
                return true;
            }
            "invite" | "pack" | "part" | "f" | "attack" | "e" | "talk" | "form" | "channel"
            | "drill" | "technique" => match self.target(argument) {
                Ok(id) => match command {
                    "f" | "attack" => Intent::Attack(id),
                    "form" => Intent::ReedStrike(id),
                    "channel" => {
                        if !self.world.is_sect_cultivation() {
                            self.notice = "Only in C3.0.".into();
                            return true;
                        }
                        Intent::ChannelStrike(id)
                    }
                    "drill" => {
                        if self.world.martial().is_none() {
                            self.notice = "Only in C3.1.".into();
                            return true;
                        }
                        Intent::Drill(id)
                    }
                    "technique" => {
                        if self.world.martial().is_none() {
                            self.notice = "Only in C3.1.".into();
                            return true;
                        }
                        Intent::TechniqueStrike(id)
                    }
                    "invite" => Intent::Invite(id),
                    "pack" => Intent::Provision(id),
                    "part" => Intent::Part(id),
                    _ => Intent::Talk(id),
                },
                Err(message) => {
                    self.notice = message.into();
                    return true;
                }
            },
            "v" | "save" => {
                self.notice = match save_living(&self.save_path, &self.world, self.locale) {
                    Ok(()) => text(self.locale, "Saved the settlement.", "Desa tersimpan.").into(),
                    Err(error) => format!(
                        "{} — {error}",
                        text(self.locale, "Save failed", "Gagal menyimpan")
                    ),
                };
                return true;
            }
            "o" | "load" => {
                self.notice = match load_campaign(
                    &self.save_path,
                    self.locale,
                    self.world.checkpoint_revision(),
                ) {
                    Ok(world) => {
                        self.world = world;
                        text(
                            self.locale,
                            "Restored the settlement.",
                            "Desa berhasil dimuat.",
                        )
                        .into()
                    }
                    Err(error) => format!(
                        "{} — {error}",
                        text(self.locale, "Load failed", "Gagal memuat")
                    ),
                };
                return true;
            }
            "r" | "reset" => {
                let seed = self.world.region().map_or(0, |region| region.seed);
                self.world = self.new_seeded_world(seed);
                self.notice = text(
                    self.locale,
                    "Started a new settlement.",
                    "Memulai desa baru.",
                )
                .into();
                return true;
            }
            "journal" => {
                let lines: Vec<_> = self
                    .world
                    .company()
                    .into_iter()
                    .flat_map(|company| company.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.company_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No recorded travel encounters.",
                        "Belum ada catatan perjalanan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "trafficlog" => {
                let lines: Vec<_> = self
                    .world
                    .traffic()
                    .into_iter()
                    .flat_map(|traffic| traffic.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.traffic_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed passage exchanges.",
                        "Belum ada pertukaran jalan yang disaksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "relieflog" => {
                let lines: Vec<_> = self
                    .world
                    .relief()
                    .into_iter()
                    .flat_map(|relief| relief.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.relief_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed relief events.",
                        "Belum ada kejadian bantuan yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "aidlog" => {
                let lines: Vec<_> = self
                    .world
                    .faction_agency_state()
                    .into_iter()
                    .flat_map(|agency| agency.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.faction_aid_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed faction aid events.",
                        "Belum ada bantuan antarfaksi yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "contractlog" => {
                let lines: Vec<_> = self
                    .world
                    .contracts()
                    .into_iter()
                    .flat_map(|registry| registry.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.contract_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed institutional contracts.",
                        "Belum ada kontrak lembaga yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "accountlog" => {
                let lines: Vec<_> = self
                    .world
                    .contract_memory()
                    .into_iter()
                    .flat_map(|memory| memory.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.account_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed contract audits.",
                        "Belum ada pemeriksaan kontrak yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "infolog" => {
                let lines: Vec<_> = self
                    .world
                    .information()
                    .into_iter()
                    .flat_map(|info| info.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| self.info_line(event))
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed institutional information.",
                        "Belum ada informasi lembaga yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "factions" => {
                self.notice = self.faction_status();
                return true;
            }
            "factionlog" => {
                let lines: Vec<_> = self
                    .world
                    .rivalry()
                    .into_iter()
                    .flat_map(|rival| rival.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| {
                        let verb = match event.kind {
                            SectEventKind::Purchased => {
                                text(self.locale, "bought an herb", "membeli herba")
                            }
                            SectEventKind::Petitioned => text(
                                self.locale,
                                "pressed a claim at Reed Hall",
                                "menuntut pengaruh di Balai Alang-alang",
                            ),
                            SectEventKind::Accord => {
                                text(self.locale, "negotiated a truce", "menegosiasikan gencatan")
                            }
                            SectEventKind::Patronage => text(
                                self.locale,
                                "supported Reed Hall",
                                "mendukung Balai Alang-alang",
                            ),
                        };
                        format!(
                            "t{} · {} {verb}.",
                            event.at,
                            actor_name(self.locale, &self.world, event.actor)
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "You witnessed no sect events.",
                        "Belum ada peristiwa perguruan yang kamu saksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "politicslog" => {
                let lines: Vec<_> = self
                    .world
                    .politics()
                    .into_iter()
                    .flat_map(|politics| politics.history.iter().rev())
                    .filter(|event| event.witnessed)
                    .take(8)
                    .map(|event| {
                        let action = match event.kind {
                            PoliticalEventKind::HallPurchase => text(
                                self.locale,
                                "bought herbs for Reed Hall",
                                "membeli herba untuk Balai Alang-alang",
                            ),
                            PoliticalEventKind::HallSupply => text(
                                self.locale,
                                "delivered Hall supplies",
                                "menyerahkan perbekalan balai",
                            ),
                            PoliticalEventKind::LodgeOath => text(
                                self.locale,
                                "pledged to Stonebank",
                                "bersumpah kepada Batu Tepi",
                            ),
                            PoliticalEventKind::LodgeAid => text(
                                self.locale,
                                "supplied Stonebank",
                                "membekali perguruan Batu Tepi",
                            ),
                            PoliticalEventKind::LodgeRenunciation => text(
                                self.locale,
                                "renounced Stonebank",
                                "melepaskan sumpah Batu Tepi",
                            ),
                            PoliticalEventKind::HallPatronage => text(
                                self.locale,
                                "supported Reed Hall",
                                "mendukung Balai Alang-alang",
                            ),
                        };
                        format!(
                            "t{} · {} {action}.",
                            event.at,
                            actor_name(self.locale, &self.world, event.actor)
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    text(
                        self.locale,
                        "No witnessed political events.",
                        "Belum ada peristiwa politik yang disaksikan.",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "look" => return true,
            _ => {
                self.notice = format!(
                    "{} — {command}",
                    text(self.locale, "Unknown command", "Perintah tidak dikenal")
                );
                return true;
            }
        };

        if let Err(error) = self.world.step(self.world.tick(), intent) {
            self.notice = match error {
                StepError::UnavailableInOriginalRules => text(
                    self.locale,
                    "Breath practice requires a new campaign.",
                    "Latihan napas tersedia di kampanye baru.",
                )
                .into(),
                StepError::RegionalCampaignRequired => text(
                    self.locale,
                    "This action requires a Qinghe–Willowford campaign.",
                    "Aksi ini memerlukan kampanye Qinghe–Willowford.",
                )
                .into(),
                StepError::CompanyCampaignRequired => text(
                    self.locale,
                    "Travelling partnerships require a new campaign.",
                    "Mulai kampanye baru untuk merekrut pendamping.",
                )
                .into(),
                StepError::RivalryCampaignRequired => text(
                    self.locale,
                    "Sect diplomacy requires a C1.5 campaign.",
                    "Diplomasi perguruan memerlukan kampanye C1.5.",
                )
                .into(),
                StepError::PoliticsCampaignRequired => text(
                    self.locale,
                    "Allegiance requires a C1.6 campaign.",
                    "Kesetiaan perguruan memerlukan kampanye C1.6.",
                )
                .into(),
                StepError::SchoolCampaignRequired => text(
                    self.locale,
                    "This action requires a new Reed Hall campaign.",
                    "Aksi ini memerlukan kampanye baru Balai Alang-alang.",
                )
                .into(),
                StepError::HorizonReached => text(
                    self.locale,
                    "This campaign reached its turn limit.",
                    "Kampanye ini mencapai batas giliran.",
                )
                .into(),
                StepError::WrongBoundary { .. } => text(
                    self.locale,
                    "The turn is out of date.",
                    "Giliran sudah berubah.",
                )
                .into(),
                StepError::InvalidBodyReport => text(
                    self.locale,
                    "Invalid body report or unavailable campaign.",
                    "Laporan jenazah tidak sah atau kampanye tidak tersedia.",
                )
                .into(),
                StepError::UnrecordableIntent => text(
                    self.locale,
                    "This command cannot be recorded exactly in this campaign.",
                    "Perintah ini tidak dapat direkam secara utuh dalam kampanye ini.",
                )
                .into(),
                StepError::ContestedCampaignRequired => text(
                    self.locale,
                    "Contested resources are unavailable in this campaign.",
                    "Barang sengketa tidak tersedia di kampanye ini.",
                )
                .into(),
            };
        }
        true
    }

    fn body(&self, argument: Option<&str>) -> Result<ActorId, &'static str> {
        let Some(text) = argument else {
            return Err(self.target_hint());
        };
        let id = text
            .parse()
            .map(ActorId::new)
            .map_err(|_| self.target_hint())?;
        self.world
            .npc(id)
            .filter(|npc| self.world.player_sees(npc.position))
            .map(|_| id)
            .ok_or(self.target_hint())
    }

    fn target(&self, argument: Option<&str>) -> Result<ActorId, &'static str> {
        if let Some(text) = argument {
            let id = text
                .parse()
                .map(ActorId::new)
                .map_err(|_| self.target_hint())?;
            return self
                .world
                .npc(id)
                .filter(|npc| npc.alive() && self.world.player_sees(npc.position))
                .map(|_| id)
                .ok_or(self.target_hint());
        }
        let player = self.world.player();
        let mut adjacent = self.world.npcs().filter(|npc| {
            npc.alive()
                && self.world.player_sees(npc.position)
                && npc.position.x.abs_diff(player.position.x)
                    + npc.position.y.abs_diff(player.position.y)
                    <= 1
        });
        match (adjacent.next(), adjacent.next()) {
            (Some(npc), None) => Ok(npc.id()),
            _ => Err(self.target_hint()),
        }
    }

    fn target_hint(&self) -> &'static str {
        text(
            self.locale,
            "Choose a visible NPC by ID.",
            "Pilih ID NPC yang terlihat.",
        )
    }

    fn inspect(&self, argument: Option<&str>) -> String {
        let id = match self.target(argument) {
            Ok(id) => id,
            Err(message) => return message.into(),
        };
        let npc = self.world.npc(id).expect("visible target");
        let tr = |en, id| text(self.locale, en, id);
        let occupation = match npc.definition.role {
            Role::Medic => tr("healer", "tabib"),
            Role::Watch => tr("watch", "penjaga"),
            Role::Supplier => tr("herbalist", "pemetik herba"),
            Role::Traveler => tr("traveler", "musafir"),
            Role::Grower => tr("grower", "pekebun"),
            Role::Courier => tr("courier", "kurir"),
            Role::Mentor => tr("mentor", "guru balai"),
            Role::HallDeputy => tr("deputy", "wakil guru"),
            Role::RivalEnvoy => tr("Stonebank envoy", "utusan perguruan Batu Tepi"),
            Role::HallSteward => tr("Hall steward", "pengurus Balai Alang-alang"),
        };
        let condition = if npc.hp < npc.definition.hp {
            tr("wounded", "terluka")
        } else {
            tr("no visible wounds", "tidak tampak luka")
        };
        if self
            .world
            .company()
            .and_then(|company| company.journey)
            .is_some_and(|j| j.actor == id)
            && npc.position.x.abs_diff(self.world.player().position.x)
                + npc.position.y.abs_diff(self.world.player().position.y)
                <= 1
        {
            return format!(
                "{} ({occupation}) — {condition}. {} {}.",
                npc.definition.name,
                tr("Packed herbs", "Herba bekal"),
                npc.packed_herbs
            );
        }
        let invitation = match self.world.invitation(id) {
            Some(Invitation::OpenRoad) => tr(
                " Talk, then invite to travel.",
                " Ajak bicara, lalu gunakan invite.",
            ),
            Some(Invitation::SchoolMember) => tr(
                " Talk first. Hall members can invite the deputy until they take office.",
                " Berkenalanlah dulu. Anggota balai bisa mengajak wakil sebelum ia menjabat guru.",
            ),
            Some(Invitation::LodgeMember) => tr(
                " Talk first. Stonebank members may invite this envoy.",
                " Berkenalanlah dulu. Anggota Batu Tepi bisa mengajak utusan ini.",
            ),
            None => "",
        };
        format!(
            "{} ({occupation}) — {condition}.{invitation}",
            npc.definition.name
        )
    }

    /// Both presentation modes project the same observation rules.
    pub(crate) fn tile_symbol(&self, cell: Position) -> char {
        let world = &self.world;
        let scenario = world.scenario();
        let player = world.player();
        let Position { x, y } = cell;
        if player.position == cell {
            return if player.alive() { '@' } else { 'x' };
        }
        if !self.debug_world && !world.player_sees(cell) {
            let wall_edge = !world.walkable(cell)
                && [
                    x.checked_sub(1).map(|x| Position { x, y }),
                    y.checked_sub(1).map(|y| Position { x, y }),
                    Some(Position { x: x + 1, y }),
                    Some(Position { x, y: y + 1 }),
                ]
                .into_iter()
                .flatten()
                .any(|neighbor| world.player_sees(neighbor));
            return if wall_edge { '#' } else { '?' };
        }
        if let Some(npc) = world.npcs().find(|npc| npc.alive() && npc.position == cell) {
            return npc.definition.glyph;
        }
        if world.npcs().any(|npc| !npc.alive() && npc.position == cell) {
            return 'x';
        }
        if cell == scenario.patch {
            return '*';
        }
        if cell == scenario.shrine {
            return '+';
        }
        if cell == scenario.clinic {
            return '=';
        }
        if world.region().is_some() {
            let sites = Region::sites();
            if cell == sites.market {
                return '$';
            }
            if cell == sites.garden {
                return '&';
            }
            if cell == sites.hall {
                return '^';
            }
        }
        if world.walkable(cell) { '.' } else { '#' }
    }

    fn frame(&self) -> String {
        let world = &self.world;
        let scenario = world.scenario();
        let player = world.player();
        let tr = |en, id| text(self.locale, en, id);
        let visible = |cell| self.debug_world || world.player_sees(cell);
        let (hour, minute) = clock::clock_hhmm(clock::minute_of_day(world.calendar_tick()));
        let mut out = format!(
            "Sealed Heaven - {}\n{} {}  {hour:02}:{minute:02}  {} {}  HP {}  {} {}{}\n",
            scenario.name,
            tr("Day", "Hari"),
            clock::day_number(world.calendar_tick()),
            tr("Turn", "Giliran"),
            world.tick(),
            player.hp,
            tr("Herbs", "Herba"),
            player.herbs,
            if player.alive() {
                ""
            } else {
                tr("  DEAD - n advances time", "  GUGUR - n memajukan waktu")
            }
        );
        if self.debug_world {
            let _ = writeln!(out, "{}", tr("DEVELOPER VIEW", "TAMPILAN PENGEMBANG"));
        }
        if world.original_rules() {
            let _ = writeln!(
                out,
                "{}",
                tr("Original C1.0 campaign", "Kampanye C1.0 awal")
            );
        } else {
            let _ = writeln!(
                out,
                "{} {}/{}",
                tr("Breath", "Napas"),
                world.breath_progress(),
                BREATH_MASTERY
            );
        }
        let sites = world.region().map(|_| Region::sites());
        if let Some(region) = world.region() {
            let _ = writeln!(
                out,
                "{} {}  |  {} {}  |  {} {}",
                tr("Coins", "Koin"),
                region.coins,
                tr("Remedies", "Obat"),
                region.remedies,
                tr("Seed", "Seed"),
                region.seed
            );
            if let Some(background) = region.background {
                let _ = writeln!(out, "{}", background_label(self.locale, background));
            } else {
                let _ = writeln!(
                    out,
                    "{}",
                    tr(
                        "Before moving, choose background wanderer (3 coins), herbalist (1 coin, up to 2 herbs per gather), or peddler (6 coins).",
                        "Sebelum bergerak, pilih background wanderer (3 koin), herbalist (1 koin, hingga 2 herba per petik), atau peddler (6 koin).",
                    )
                );
            }
            if let Some(rank) = world.hall_rank() {
                let label = hall_rank_label(self.locale, rank);
                let _ = writeln!(out, "{label}");
            }
            if world
                .politics()
                .is_some_and(|politics| politics.allegiance == Allegiance::Stonebank)
            {
                let _ = writeln!(
                    out,
                    "{}",
                    tr("Stonebank lodge member", "Anggota perguruan Batu Tepi")
                );
            }
        }
        if let Some(school) = world.school() {
            if school.form_learned {
                let _ = writeln!(out, "Qi {}/{}", school.qi, MAX_QI);
            }
            if let Some(hint) = self.duty_hint() {
                let _ = writeln!(out, "{hint}");
            }
        }
        let width = if self.debug_world {
            scenario.width
        } else {
            scenario.width.min(17)
        };
        let left = player
            .position
            .x
            .saturating_sub(width / 2)
            .min(scenario.width - width);
        for y in 0..scenario.height {
            for x in left..left + width {
                let glyph = self.tile_symbol(Position { x, y });
                let _ = write!(out, "{glyph} ");
            }
            out.push('\n');
        }
        let _ = writeln!(
            out,
            "{}",
            tr(
                "@ you  * herb patch  + shrine  = clinic  # wall  x fallen  ? unseen",
                "@ kamu  * ladang herba  + kuil  = klinik  # tembok  x gugur  ? tak terlihat",
            )
        );
        for npc in world.npcs().filter(|npc| visible(npc.position)) {
            let status = if !npc.alive() {
                tr("fallen", "gugur")
            } else if self.debug_world {
                goal_label(self.locale, npc.activity.goal)
            } else if world
                .company()
                .and_then(|company| company.journey)
                .is_some_and(|j| j.actor == npc.id())
            {
                match npc.activity.goal {
                    Goal::Rest => tr("companion resting", "pendamping beristirahat"),
                    Goal::SeekCare => tr("companion seeking care", "pendamping mencari perawatan"),
                    _ => tr("travelling companion", "pendamping perjalanan"),
                }
            } else if npc.herbs > 0 {
                tr("carrying herbs", "membawa herba")
            } else if npc.definition.role == Role::Medic && npc.position == scenario.clinic {
                tr("at the clinic", "di klinik")
            } else {
                ""
            };
            let _ = write!(
                out,
                "{} {:>2} {} ({},{})",
                npc.definition.glyph,
                npc.id().get(),
                npc.definition.name,
                npc.position.x,
                npc.position.y
            );
            if !status.is_empty() {
                let _ = write!(out, " - {status}");
            }
            out.push('\n');
        }
        let supply = world.supply();
        if visible(scenario.shrine) {
            let _ = writeln!(
                out,
                "{} {}/{}",
                tr("Shrine stock", "Stok kuil"),
                supply.shrine,
                scenario.shrine_target
            );
        }
        if visible(scenario.patch) {
            let _ = writeln!(
                out,
                "{} {}",
                tr("Herbs in patch", "Herba di ladang"),
                supply.source
            );
        }
        if let Some((region, sites)) = world.region().zip(sites) {
            if visible(sites.market) {
                let quote = world.market_quote().expect("region present");
                if world.is_night() {
                    let _ = writeln!(
                        out,
                        "{}",
                        tr(
                            "Willowford market closed until dawn",
                            "Pasar Willowford tutup hingga fajar"
                        )
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "{} {}  |  {} {}  |  {} {}",
                        tr("Market herbs", "Herba pasar"),
                        region.market_herbs,
                        tr("buy", "beli"),
                        quote.buy,
                        tr("sell", "jual"),
                        quote.sell
                    );
                }
            }
            if visible(sites.garden) {
                let _ = writeln!(
                    out,
                    "{} {}",
                    tr("Willowford garden herbs", "Herba kebun Willowford"),
                    region.garden_herbs
                );
            }
            if visible(sites.hall) {
                let _ = writeln!(out, "{}", self.hall_hint());
                if let Some(rivalry) = world.rivalry() {
                    let state = if rivalry.at_peace(world.tick()) {
                        tr("truce", "gencatan")
                    } else if rivalry.contested(world.tick()) {
                        tr("contested", "diperebutkan")
                    } else {
                        tr("stable", "stabil")
                    };
                    let _ = writeln!(out, "{}: {state}", tr("Hall influence", "Pengaruh balai"));
                }
            }
            if self.debug_world {
                let _ = writeln!(
                    out,
                    "{} {}",
                    tr("Herbs shipped to Qinghe", "Herba dikirim ke Qinghe"),
                    region.shipped_to_qinghe
                );
            }
        }
        if self.debug_world {
            let _ = writeln!(
                out,
                "{} {}  |  {} {}",
                tr("NPC deliveries", "Kiriman NPC"),
                supply.npc_deliveries,
                tr("Used for care", "Dipakai untuk perawatan"),
                supply.consumed
            );
        }
        if let Some(outcome) = world.latest_outcome() {
            let _ = writeln!(out, "{}", outcome_label(self.locale, world, outcome));
        }
        for incident in world
            .social()
            .incidents()
            .iter()
            .rev()
            .take_while(|event| world.tick() - event.tick <= 3)
            .filter(|event| event.source == PLAYER || event.target == PLAYER)
            .take(3)
        {
            if incident.damage == 0 {
                let _ = writeln!(
                    out,
                    "t{} {} {} {}",
                    incident.tick,
                    actor_name(self.locale, world, incident.source),
                    tr("took cargo from", "mengambil muatan dari"),
                    actor_name(self.locale, world, incident.target),
                );
            } else {
                let _ = writeln!(
                    out,
                    "t{} {} {} {} ({} HP){}",
                    incident.tick,
                    actor_name(self.locale, world, incident.source),
                    tr("hit", "menyerang"),
                    actor_name(self.locale, world, incident.target),
                    incident.damage,
                    if incident.fatal {
                        tr(" fatal", " fatal")
                    } else {
                        ""
                    }
                );
            }
        }
        if let Some(company) = world.company() {
            for event in company
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| {
                    event.witnessed
                        && !matches!(event.kind, JourneyEvent::Joined | JourneyEvent::Dismissed)
                })
            {
                let _ = writeln!(out, "{}", self.company_line(event));
            }
        }
        if let Some(traffic) = world.traffic() {
            for event in traffic
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.traffic_line(event));
            }
        }
        if let Some(relief) = world.relief() {
            for event in relief
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.relief_line(event));
            }
        }
        if let Some(agency) = world.faction_agency_state() {
            for event in agency
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.faction_aid_line(event));
            }
        }
        if let Some(registry) = world.contracts() {
            for event in registry
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.contract_line(event));
            }
        }
        if let Some(memory) = world.contract_memory() {
            for event in memory
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.account_line(event));
            }
        }
        if let Some(information) = world.information() {
            for event in information
                .history
                .iter()
                .rev()
                .take_while(|event| event.at == world.tick())
                .filter(|event| event.witnessed)
            {
                let _ = writeln!(out, "{}", self.info_line(event));
            }
        }
        if !self.notice.is_empty() {
            let _ = writeln!(out, "{}", self.notice);
        }
        let _ = writeln!(
            out,
            "\n{}",
            tr(
                "wasd move | n wait | f [id] attack | b guard | e [id] talk or request care | i [id] inspect",
                "wasd bergerak | n tunggu | f [id] serang | b bertahan | e [id] bicara atau berobat | i [id] amati",
            )
        );
        let _ = writeln!(
            out,
            "{}",
            tr(
                "g gather | h deliver | v save | o load | r reset | q quit | lang id",
                "g petik | h sumbang | v simpan | o muat | r ulang | q keluar | lang en",
            )
        );
        if !world.original_rules() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "m practice at the shrine (costs 1 herb, mastery improves guard)",
                    "m latihan di kuil (1 herba, penguasaan memperkuat pertahanan)",
                )
            );
        }
        if sites.is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "$ market  & garden  ^ hall | buy | sell | join | brew | remedy | help",
                    "$ pasar  & kebun  ^ balai | buy beli | sell jual | join | brew racik | remedy minum | help",
                )
            );
        }
        if world.school().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "duty carry or report | study learn form | center gain Qi | form [id] strike",
                    "duty antar atau lapor | study belajar jurus | center pulihkan Qi | form [id] serang",
                )
            );
        }
        if world.company().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "invite [id] travel together | pack [id] give a medicinal herb | part [id] | journal",
                    "invite [id] ajak ikut | pack [id] bekali satu herba | part [id] berpisah | journal catatan",
                )
            );
        }
        if world.rivalry().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "V envoy | accord (offer 1 herb) | sponsor (Hall member, 1 herb) | factions | factionlog",
                    "V utusan | accord (beri 1 herba) | sponsor (anggota balai, 1 herba) | factions | factionlog",
                )
            );
        }
        if world.politics().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "Q Hall steward | pledge (talk to envoy, 1 herb) | aidlodge (1 herb) | renounce (at Hall) | politicslog",
                    "Q pengurus balai | pledge (bicara utusan, 1 herba) | aidlodge (1 herba) | renounce (di balai) | politicslog",
                )
            );
        }
        if world.traffic().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "Move toward friendly NPC to pass | trafficlog records witnessed exchanges",
                    "Bergerak ke NPC ramah untuk lewat | trafficlog mencatat pertukaran yang terlihat",
                )
            );
        }
        if world.relief().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "relieflog: witnessed Qinghe crisis and Reed Hall assistance",
                    "relieflog: krisis Qinghe dan bantuan balai yang kamu saksikan",
                )
            );
        }
        if world.faction_agency_state().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "aidlog: witnessed Stonebank surveys, physical relief and local support",
                    "aidlog: peninjauan Batu Tepi, bantuan fisik dan dukungan warga yang terlihat",
                )
            );
        }
        if world.contracts().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "contractlog: witnessed institutional requests, bids, awards and physical deliveries",
                    "contractlog: permintaan, tawaran, persetujuan dan pengiriman lembaga yang disaksikan",
                )
            );
        }
        if world.contract_memory().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "accountlog: witnessed site audits and provider reliability learned in person",
                    "accountlog: pemeriksaan lokasi dan keandalan penyedia yang terlihat langsung",
                )
            );
        }
        if world.information().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "infolog: witnessed testimony and institutional disputes you were present for",
                    "infolog: kesaksian dan sengketa lembaga yang kamu saksikan langsung",
                )
            );
        }
        if world.is_consequences() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "know | examine | ask [id] | tell [id] [event] | take [id] from a body you can see",
                    "know | examine | ask [id] | tell [id] [peristiwa] | take [id] dari jenazah yang terlihat",
                )
            );
        } else if world.is_player_agency() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "know lists what you know | examine this tile | ask [id] | tell [id] [event]",
                    "know daftar pengetahuan | examine petak ini | ask [id] | tell [id] [peristiwa]",
                )
            );
        }
        out
    }

    fn faction_status(&self) -> String {
        let tr = |en, id| text(self.locale, en, id);
        let Some(rivalry) = self.world.rivalry() else {
            return tr(
                "No active sect dispute in this campaign.",
                "Tidak ada konflik perguruan di kampanye ini.",
            )
            .into();
        };
        let sites = Region::sites();
        if !self.debug_world
            && !self.world.player_sees(sites.market)
            && !self.world.player_sees(sites.hall)
        {
            return tr(
                "Ask in Willowford to learn about the local dispute.",
                "Cari kabar di Willowford untuk mengetahui konflik setempat.",
            )
            .into();
        }
        let condition = if rivalry.at_peace(self.world.tick()) {
            tr("Truce", "Gencatan")
        } else if rivalry.contested(self.world.tick()) {
            tr(
                "Reed Hall contested: teaching suspended and herbs dearer",
                "Balai diperebutkan: pengajaran tertunda dan herba lebih mahal",
            )
        } else {
            tr("Reed Hall still teaching", "Balai masih mengajar")
        };
        let mut public = format!(
            "{condition}. {} {}/8.",
            tr("Pressure", "Tekanan"),
            rivalry.pressure
        );
        if let Some(politics) = self.world.politics() {
            let oath = if politics.allegiance == Allegiance::Stonebank {
                tr("Stonebank member", "Anggota Batu Tepi")
            } else {
                tr("Unaffiliated", "Tidak berafiliasi")
            };
            public.push_str(&format!(" {oath}."));
            public.push_str(&format!(
                " {} {} / {} {}.",
                tr("Hall favor", "Dukungan balai"),
                politics.hall_favor,
                tr("Lodge favor", "Dukungan Batu Tepi"),
                politics.lodge_favor
            ));
        }
        if self.debug_world
            && let Some(agency) = self.world.faction_agency_state()
        {
            public.push_str(&format!(
                " {} {}.",
                tr(
                    "Village support (Hall + / Stonebank -)",
                    "Dukungan warga (Balai + / Batu Tepi -)"
                ),
                agency.public_standing
            ));
        }
        if self.debug_world {
            let mut details = format!(
                "{public} {} {}.",
                tr("Lodge treasury", "Kas perguruan rival"),
                rivalry.lodge_coins
            );
            if let Some(politics) = self.world.politics() {
                details.push_str(&format!(
                    " {} {}.",
                    tr("Hall treasury", "Kas balai"),
                    politics.hall_coins
                ));
            }
            details
        } else {
            public
        }
    }

    pub(crate) fn company_line(&self, event: &CompanyEvent) -> String {
        let tr = |en, id| text(self.locale, en, id);
        let action = match event.kind {
            JourneyEvent::Joined => tr("joined you", "mulai menemanimu"),
            JourneyEvent::Dismissed => tr("parted ways with you", "berpisah denganmu"),
            JourneyEvent::LostContact => tr("lost the trail", "kehilangan jejak"),
            JourneyEvent::Distrust => tr(
                "refused to continue with you",
                "menolak melanjutkan perjalanan bersamamu",
            ),
            JourneyEvent::Office => tr(
                "left to take up the teaching post",
                "kembali untuk menjabat guru",
            ),
            JourneyEvent::Fallen => tr("fell", "gugur"),
            JourneyEvent::PlayerFallen => tr("ended the journey", "mengakhiri perjalanan"),
            JourneyEvent::FieldCare => tr(
                "used a packed herb to treat you",
                "memakai satu herba bekal untuk mengobatimu",
            ),
        };
        format!(
            "t{} · {} {action}.",
            event.at,
            actor_name(self.locale, &self.world, event.actor)
        )
    }

    fn traffic_line(&self, event: &sealed_sim::living::TrafficEvent) -> String {
        let action = match event.kind {
            TrafficEventKind::PlayerPass => text(
                self.locale,
                "exchanged places with",
                "bertukar tempat dengan",
            ),
            TrafficEventKind::NpcPass => text(self.locale, "passed", "melewati"),
        };
        format!(
            "t{} · {} {action} {} ({},{} -> {},{}).",
            event.at,
            actor_name(self.locale, &self.world, event.mover),
            actor_name(self.locale, &self.world, event.other),
            event.from.x,
            event.from.y,
            event.to.x,
            event.to.y
        )
    }

    fn relief_line(&self, event: &sealed_sim::living::ReliefEvent) -> String {
        let action = match event.kind {
            ReliefEventKind::Observed => text(
                self.locale,
                "noticed an urgent medicine shortage",
                "melihat kekurangan obat yang mendesak",
            ),
            ReliefEventKind::Reported => text(
                self.locale,
                "asked Reed Hall for aid",
                "meminta bantuan Balai Alang-alang",
            ),
            ReliefEventKind::Purchased => text(
                self.locale,
                "bought an herb for Qinghe",
                "membeli herba untuk Qinghe",
            ),
            ReliefEventKind::Redirected => text(
                self.locale,
                "redirected an herb to Qinghe",
                "mengalihkan satu herba ke Qinghe",
            ),
            ReliefEventKind::Delivered => text(
                self.locale,
                "delivered medical relief to Qinghe",
                "mengantar bantuan obat ke Qinghe",
            ),
            ReliefEventKind::Withdrawn => text(
                self.locale,
                "stopped a no-longer-needed relief mission",
                "menghentikan misi bantuan yang tak lagi diperlukan",
            ),
            ReliefEventKind::Failed => text(
                self.locale,
                "lost a relief mission",
                "gagal menuntaskan misi bantuan",
            ),
        };
        format!(
            "t{} · {} {action}.",
            event.at,
            actor_name(self.locale, &self.world, event.actor)
        )
    }

    fn faction_aid_line(&self, event: &sealed_sim::living::FactionEvent) -> String {
        let action = match event.kind {
            FactionEventKind::SurveyClear => text(
                self.locale,
                "found Qinghe supplied",
                "menemukan Qinghe berkecukupan",
            ),
            FactionEventKind::ShortageSeen => text(
                self.locale,
                "personally witnessed Qinghe's medicine shortage",
                "menyaksikan sendiri kekurangan obat Qinghe",
            ),
            FactionEventKind::Purchased => text(
                self.locale,
                "bought a real herb for Stonebank relief",
                "membeli herba nyata untuk bantuan Batu Tepi",
            ),
            FactionEventKind::Redirected => text(
                self.locale,
                "redirected personal cargo to Qinghe",
                "mengalihkan muatan pribadi ke Qinghe",
            ),
            FactionEventKind::Delivered => text(
                self.locale,
                "delivered Stonebank medicine to Qinghe",
                "mengantar obat Batu Tepi ke Qinghe",
            ),
            FactionEventKind::HallDelivered => text(
                self.locale,
                "delivered Reed Hall medicine to Qinghe",
                "mengantar obat Balai Alang-alang ke Qinghe",
            ),
            FactionEventKind::Withdrawn => text(
                self.locale,
                "left the aid mission to honor a companion",
                "meninggalkan misi bantuan demi pendamping",
            ),
            FactionEventKind::Failed => text(
                self.locale,
                "failed to finish the Stonebank relief trip",
                "gagal menuntaskan perjalanan bantuan Batu Tepi",
            ),
        };
        format!(
            "t{} · {} {action}.",
            event.at,
            actor_name(self.locale, &self.world, event.actor)
        )
    }

    fn contract_line(&self, event: &sealed_sim::living::ContractEvent) -> String {
        let objective = match event.need {
            Need::ShrineMedicine => text(self.locale, "Qinghe medicine", "obat Qinghe"),
            Need::HallOfferings => text(
                self.locale,
                "Reed Hall supplies",
                "persediaan Balai Alang-alang",
            ),
        };
        let verb = match event.kind {
            ContractEventKind::Observed => {
                text(self.locale, "noticed a need for", "melihat kebutuhan")
            }
            ContractEventKind::Offered => {
                text(self.locale, "offered to help with", "menawarkan bantuan")
            }
            ContractEventKind::Declined => text(self.locale, "declined", "menolak"),
            ContractEventKind::Awarded => text(
                self.locale,
                "awarded the contract for",
                "menetapkan kontrak",
            ),
            ContractEventKind::Purchased => text(
                self.locale,
                "bought real cargo for",
                "membeli muatan nyata untuk",
            ),
            ContractEventKind::Redirected => text(
                self.locale,
                "redirected existing cargo to",
                "mengalihkan muatan yang ada ke",
            ),
            ContractEventKind::Delivered => text(
                self.locale,
                "physically delivered",
                "mengantar secara fisik",
            ),
            ContractEventKind::Failed => {
                text(self.locale, "could not finish", "gagal menyelesaikan")
            }
            ContractEventKind::Withdrawn => text(self.locale, "withdrew from", "meninggalkan"),
            ContractEventKind::ResolvedLocally => text(
                self.locale,
                "resolved locally before awarding",
                "diselesaikan setempat sebelum penunjukan",
            ),
        };
        format!(
            "t{} #{} · {} {verb} {objective}.",
            event.at,
            event.case_id,
            actor_name(self.locale, &self.world, event.actor)
        )
    }

    fn account_line(&self, event: &sealed_sim::living::AuditEvent) -> String {
        let verdict = match event.finding {
            AuditFinding::Delivered => text(
                self.locale,
                "verified a signed delivery by",
                "memastikan pengiriman tercatat oleh",
            ),
            AuditFinding::Unfulfilled => text(
                self.locale,
                "found no delivery by the deadline from",
                "tidak menemukan kiriman hingga tenggat dari",
            ),
        };
        format!(
            "t{} #{} · {} {verdict} {}.",
            event.at,
            event.case_id,
            actor_name(self.locale, &self.world, event.requester),
            actor_name(self.locale, &self.world, event.provider),
        )
    }

    fn info_line(&self, event: &sealed_sim::living::InfoLogEvent) -> String {
        let actor = actor_name(self.locale, &self.world, event.actor);
        let other = event
            .other
            .map(|id| actor_name(self.locale, &self.world, id))
            .unwrap_or_else(|| text(self.locale, "someone", "seseorang"));
        let phase = event.phase.map(|phase| match phase {
            DisputePhase::Open => text(self.locale, "opened", "dibuka"),
            DisputePhase::Investigating => text(self.locale, "investigating", "diselidiki"),
            DisputePhase::Confirmed => text(self.locale, "confirmed", "dikonfirmasi"),
            DisputePhase::Refuted => text(self.locale, "refuted", "dibantah"),
            DisputePhase::Suspended => text(self.locale, "suspended", "ditangguhkan"),
            DisputePhase::Restored => text(self.locale, "restored", "dipulihkan"),
        });
        let tick = event.at.to_string();
        let id = event.event.0.to_string();
        match event.kind {
            InfoLogKind::Learned => text(
                self.locale,
                "t{tick} · {actor} learned institutional finding #{id}.",
                "t{tick} · {actor} mempelajari temuan lembaga #{id}.",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{id}", &id),
            InfoLogKind::DisputeOpened => text(
                self.locale,
                "t{tick} · {actor} {verb} a dispute about {other}.",
                "t{tick} · {actor} {verb} sengketa tentang {other}.",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace(
                "{verb}",
                phase.unwrap_or(text(self.locale, "opened", "dibuka")),
            )
            .replace("{other}", other),
            InfoLogKind::DisputeUpdated => text(
                self.locale,
                "t{tick} · {actor} marked the dispute about {other} as {status}.",
                "t{tick} · {actor} menandai sengketa tentang {other} sebagai {status}.",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{other}", other)
            .replace(
                "{status}",
                phase.unwrap_or(text(self.locale, "updated", "diperbarui")),
            ),
            InfoLogKind::TestimonyQueued => text(
                self.locale,
                "t{tick} · {actor} began briefing {other} about finding #{id}.",
                "t{tick} · {actor} mulai memberi tahu {other} tentang temuan #{id}.",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{other}", other)
            .replace("{id}", &id),
        }
    }

    pub(crate) fn duty_hint(&self) -> Option<&'static str> {
        let tr = |en, id| text(self.locale, en, id);
        match self.world.school()?.duty {
            Duty::Carrying => Some(tr(
                "Hall parcel: use duty at the Qinghe shrine to deliver it.",
                "Titipan balai: gunakan duty di kuil Qinghe untuk menyerahkannya.",
            )),
            Duty::Delivered => Some(tr(
                "Hall parcel delivered. Return to the teacher and use duty to report.",
                "Titipan sudah sampai. Kembali ke guru dan gunakan duty untuk melapor.",
            )),
            _ => None,
        }
    }

    pub(crate) fn hall_hint(&self) -> String {
        let tr = |en, id| text(self.locale, en, id);
        if self
            .world
            .politics()
            .is_some_and(|politics| politics.allegiance == Allegiance::Stonebank)
        {
            return tr("Stonebank oath blocks Reed Hall teaching. Renounce here to seek readmission.",
                "Sumpah Batu Tepi menutup pengajaran Reed Hall. Lepaskan sumpah di sini untuk mendaftar lagi.").into();
        }
        if self
            .world
            .rivalry()
            .is_some_and(|rivalry| rivalry.contested(self.world.tick()))
        {
            return tr("Reed Hall teaching disrupted. Offer accord to the envoy or sponsor the hall.",
                "Pengajaran balai terganggu. Tawarkan accord kepada utusan atau dukung balai dengan sponsor.").into();
        }
        let Some(school) = self.world.school() else {
            return tr(
                "Reed Hall · join or brew",
                "Balai Alang-alang · join atau brew",
            )
            .into();
        };
        let Some(leader) = school.leader.and_then(|id| self.world.npc(id)) else {
            return tr(
                "The teaching post is empty.",
                "Tempat mengajar di balai kosong.",
            )
            .into();
        };
        if self.world.is_night() || leader.position != Region::sites().hall {
            return tr(
                "The hall's teacher is away.",
                "Guru balai sedang tidak bertugas.",
            )
            .into();
        }
        let prompt = match self.world.hall_rank() {
            None => tr("join after practicing breath", "join setelah latihan napas"),
            Some(HallRank::Apprentice) => tr(
                "duty for shrine service, brew for a remedy",
                "duty untuk tugas kuil, brew untuk meracik",
            ),
            Some(HallRank::Disciple) if !school.form_learned => tr(
                "study with mastered breath and 2 herbs",
                "study dengan latihan napas tuntas dan 2 herba",
            ),
            Some(HallRank::Disciple) if self.world.sect_hierarchy().is_some() => tr(
                "duty for senior rank, brew for a remedy",
                "duty untuk naik pangkat senior, brew untuk meracik",
            ),
            Some(HallRank::Disciple) => tr("brew a remedy", "brew untuk meracik obat"),
            Some(HallRank::SeniorDisciple | HallRank::Elder) => tr(
                "allocate for Hall herb, duty or brew",
                "allocate untuk herba balai, duty atau brew",
            ),
        };
        if let Some(record) = school
            .history
            .last()
            .filter(|record| record.successor == Some(leader.id()))
        {
            return format!(
                "{} {} {} · {prompt}",
                leader.definition.name,
                tr("succeeded", "menggantikan"),
                actor_name(self.locale, &self.world, record.former)
            );
        }
        format!("{} · {prompt}", leader.definition.name)
    }
}

fn save_living(path: &Path, world: &World, locale: Locale) -> Result<(), String> {
    let bytes = world.checkpoint();
    World::restore(&bytes).map_err(|error| format!("{error:?}"))?;
    match File::open(path) {
        Ok(file) => {
            let old = persistence::read_limited(file, MAX_SAVE_BYTES)?;
            let previous = World::restore(&old).map_err(|error| format!("{error:?}"))?;
            if previous.checkpoint_revision() != world.checkpoint_revision() {
                return Err(text(
                    locale,
                    "Existing save belongs to another campaign.",
                    "Simpanan yang ada milik kampanye lain.",
                )
                .into());
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    persistence::write_atomic(path, &bytes)
}

fn load_campaign(path: &Path, locale: Locale, expected_revision: u16) -> Result<World, String> {
    let world = load(path, locale)?;
    if world.checkpoint_revision() != expected_revision {
        return Err(text(
            locale,
            "Save belongs to another campaign.",
            "Simpanan milik kampanye lain.",
        )
        .into());
    }
    Ok(world)
}

fn load(path: &Path, locale: Locale) -> Result<World, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let bytes = persistence::read_limited(file, MAX_SAVE_BYTES)?;
    World::restore(&bytes).map_err(|error| match error {
        LoadError::LegacyCheckpoint => text(
            locale,
            "For a C0.9 save, use --c09 and its original save location.",
            "Untuk simpanan C0.9, gunakan --c09 dan lokasi simpan aslinya.",
        )
        .into(),
        _ => format!("{error:?}"),
    })
}

pub(crate) fn text<'a>(locale: Locale, en: &'a str, id: &'a str) -> &'a str {
    match locale {
        Locale::English => en,
        Locale::Indonesian => id,
    }
}

fn actor_name(locale: Locale, world: &World, id: ActorId) -> &'static str {
    if id == PLAYER {
        text(locale, "You", "Kamu")
    } else {
        world
            .npc(id)
            .map_or(text(locale, "Unknown", "Tak dikenal"), |npc| {
                npc.definition.name
            })
    }
}

fn background_label(locale: Locale, background: Background) -> &'static str {
    match background {
        Background::Wanderer => text(locale, "Wanderer", "Pengembara"),
        Background::Herbalist => text(locale, "Herbalist", "Pemetik herba"),
        Background::Peddler => text(locale, "Peddler", "Pedagang keliling"),
    }
}

fn goal_label(locale: Locale, goal: Goal) -> &'static str {
    let (en, id) = match goal {
        Goal::Idle => ("idle", "diam"),
        Goal::Rest => ("resting or heading home", "beristirahat atau pulang"),
        Goal::Patrol => ("patrolling", "berpatroli"),
        Goal::TendClinic => ("staffing the clinic", "menjaga klinik"),
        Goal::CollectHerbs => ("collecting herbs", "mencari herba"),
        Goal::DeliverHerbs => ("hauling herbs to the shrine", "mengantar herba ke kuil"),
        Goal::SeekCare => ("seeking care", "mencari perawatan"),
        Goal::Treat(_) => ("attending to a patient", "merawat pasien"),
        Goal::Investigate(_) => ("investigating a disturbance", "menyelidiki gangguan"),
        Goal::InvestigateBody(_) => ("checking a reported body", "memeriksa laporan jenazah"),
        Goal::RecoverBody(_) => ("recovering abandoned cargo", "mengambil muatan terlantar"),
        Goal::ReturnRecoveredCargo => ("returning recovered herbs", "mengantar herba temuan"),
        Goal::ClaimLot(_) => ("seeking a relic lot", "mencari barang relik"),
        Goal::DeliverLot(_) => (
            "carrying a relic to the sect",
            "mengantar relik ke perguruan",
        ),
        Goal::MartialPractice(_) => ("training with a mentor", "berlatih bersama guru"),
        Goal::MartialRest => (
            "recovering from training",
            "memulihkan diri setelah latihan",
        ),
        Goal::MeridianDraw => (
            "drawing Qi from an offering",
            "menyerap Qi dari persembahan",
        ),
        Goal::MeridianCirculate => ("circulating Qi", "mengalirkan Qi"),
        Goal::MeridianCare => ("seeking meridian care", "mencari perawatan meridian"),
        Goal::SeekWoundCare => ("seeking wound care", "mencari perawatan luka"),
        Goal::GatherForCare => (
            "working to afford treatment",
            "bekerja untuk biaya perawatan",
        ),
        Goal::SellForCare => (
            "selling work herbs for treatment",
            "menjual herba kerja untuk perawatan",
        ),
        Goal::SectDuty => (
            "carrying out Reed Hall service",
            "menjalankan tugas Balai Alang-alang",
        ),
        Goal::Report { .. } => ("seeking someone to report to", "mencari penerima laporan"),
        Goal::Retreat => ("retreating", "mundur"),
        Goal::Retaliate => ("retaliating", "membalas serangan"),
        Goal::StockGarden => ("collecting garden herbs", "memetik herba kebun"),
        Goal::StockMarket => ("stocking Willowford market", "mengisi pasar Willowford"),
        Goal::CollectFreight => ("picking up Qinghe's order", "mengambil pesanan Qinghe"),
        Goal::DeliverFreight => ("carrying Qinghe's order", "mengantar pesanan Qinghe"),
        Goal::Follow(_) => ("following the trail", "mengikuti jejak"),
        Goal::FieldCare => ("giving field care", "merawat di perjalanan"),
        Goal::TeachAtHall => ("teaching at Reed Hall", "mengajar di Balai Alang-alang"),
        Goal::BuyForLodge => (
            "buying herbs for Stonebank lodge",
            "membeli herba untuk perguruan Batu Tepi",
        ),
        Goal::PetitionHall => (
            "pressing Stonebank's claim",
            "mengajukan tuntutan perguruan Batu Tepi",
        ),
        Goal::BuyForHall => (
            "buying herbs for Reed Hall",
            "membeli herba untuk Balai Alang-alang",
        ),
        Goal::SupplyHall => (
            "delivering herbs to Reed Hall",
            "mengantar herba untuk Balai Alang-alang",
        ),
        Goal::RequestRelief => (
            "carrying a plea for medical relief",
            "membawa permohonan bantuan obat",
        ),
        Goal::BuyRelief => ("procuring crisis medicine", "membeli obat untuk krisis"),
        Goal::DeliverRelief => ("carrying emergency medicine", "mengantar obat darurat"),
        Goal::CampRest => ("resting on the road", "beristirahat di perjalanan"),
        Goal::SurveyQinghe => (
            "surveying Qinghe for Stonebank",
            "meninjau Qinghe untuk Batu Tepi",
        ),
        Goal::BuyLodgeAid => (
            "buying Stonebank relief medicine",
            "membeli obat bantuan Batu Tepi",
        ),
        Goal::DeliverLodgeAid => (
            "carrying Stonebank relief medicine",
            "mengantar obat bantuan Batu Tepi",
        ),
        Goal::NegotiateContract(_) => (
            "negotiating an institutional contract",
            "merundingkan kontrak lembaga",
        ),
        Goal::ProcureContract(_) => ("procuring contract supplies", "membeli persediaan kontrak"),
        Goal::FulfillContract(_) => (
            "delivering contract supplies",
            "mengantar persediaan kontrak",
        ),
        Goal::AuditContract(_) => (
            "inspecting the delivery site",
            "memeriksa lokasi pengiriman",
        ),
        Goal::DeliverTestimony { .. } => (
            "carrying institutional testimony",
            "membawa kesaksian lembaga",
        ),
        Goal::InvestigateDispute(_) => (
            "investigating an institutional dispute",
            "menyelidiki sengketa lembaga",
        ),
    };
    text(locale, en, id)
}

pub(crate) fn outcome_label(locale: Locale, world: &World, outcome: Outcome) -> String {
    let tr = |en, id| text(locale, en, id);
    let (en, id) = match outcome {
        Outcome::JoinedCompany(id) | Outcome::Parted(id) | Outcome::Provisioned(id) => {
            let action = match outcome {
                Outcome::JoinedCompany(_) => tr("agreed to travel with you", "bersedia menemanimu"),
                Outcome::Parted(_) => tr("parted ways with you", "berpisah denganmu"),
                _ => tr(
                    "accepted a medicinal herb",
                    "menerima satu herba untuk pengobatan",
                ),
            };
            return format!("{} {action}.", actor_name(locale, world, id));
        }
        Outcome::Waited => ("Time passed.", "Waktu berlalu."),
        Outcome::Moved => ("You moved.", "Kamu berpindah."),
        Outcome::Spoke(id) => {
            return format!(
                "{} {}.",
                tr("You spoke with", "Kamu berbicara dengan"),
                actor_name(locale, world, id)
            );
        }
        Outcome::Gathered => ("You gathered an herb.", "Kamu memetik satu herba."),
        Outcome::GatheredPair => ("You gathered two herbs.", "Kamu memetik dua herba."),
        Outcome::DutyAccepted => (
            "You took one herb from the hall for Qinghe.",
            "Kamu menerima satu herba titipan untuk Qinghe.",
        ),
        Outcome::DutyDelivered => (
            "The hall's herb entered shrine stock.",
            "Herba titipan balai masuk ke stok kuil.",
        ),
        Outcome::DutyReported => {
            if world.hall_rank() == Some(HallRank::SeniorDisciple) {
                (
                    "The teacher recorded your merit; you stand as a senior disciple.",
                    "Guru mencatat jasamu; pangkatmu kini murid senior.",
                )
            } else {
                (
                    "The teacher recognized you as a disciple.",
                    "Guru mengangkatmu menjadi murid dalam.",
                )
            }
        }
        Outcome::FormLearned => (
            "You learned Reed Strike.",
            "Kamu mempelajari Jurus Alang-alang.",
        ),
        Outcome::LodgeAccord { until } => {
            return format!(
                "{} {until}.",
                tr(
                    "Stonebank agreed to cease petitions through turn",
                    "Perguruan Batu Tepi menyetujui gencatan sampai giliran"
                )
            );
        }
        Outcome::HallSupported => (
            "Your herb strengthened Reed Hall's standing.",
            "Herbamu memperkuat kedudukan Balai Alang-alang.",
        ),
        Outcome::LodgePledged => (
            "You pledged to Stonebank and left Reed Hall.",
            "Kamu bersumpah kepada Batu Tepi dan keluar dari Balai Alang-alang.",
        ),
        Outcome::LodgeAided => (
            "You provisioned Stonebank and strengthened its claim.",
            "Kamu membekali Batu Tepi dan memperkuat tuntutannya.",
        ),
        Outcome::LodgeRenounced => (
            "You renounced Stonebank at Reed Hall.",
            "Kamu melepas sumpah Batu Tepi di Balai Alang-alang.",
        ),
        Outcome::Learned(event) => {
            return format!(
                "{} #{}.",
                tr("You examined evidence", "Kamu memeriksa bukti"),
                event.0
            );
        }
        Outcome::Briefed { from, event } => {
            return format!(
                "{} {} #{}.",
                actor_name(locale, world, from),
                tr("will tell you about finding", "akan menyampaikan temuan"),
                event.0
            );
        }
        Outcome::Testified { to, event } => {
            return format!(
                "{} {} #{}.",
                tr("You offered finding", "Kamu menyampaikan temuan"),
                event.0,
                actor_name(locale, world, to)
            );
        }
        Outcome::Took { from, herbs } => {
            return format!(
                "{} {herbs} {} {}.",
                tr("You took", "Kamu mengambil"),
                tr("herbs from", "herba dari"),
                actor_name(locale, world, from)
            );
        }
        Outcome::BodyReported { to, body } => {
            return format!(
                "{} {} {} {}.",
                tr("You reported", "Kamu melaporkan"),
                actor_name(locale, world, body),
                tr("to", "kepada"),
                actor_name(locale, world, to)
            );
        }
        Outcome::LotClaimed(id) => {
            return format!("{} #{id}.", tr("You claimed lot", "Kamu mengambil barang"));
        }
        Outcome::LotAttuned(id) => {
            return format!("{} #{id}.", tr("You attuned lot", "Kamu mengolah barang"));
        }
        Outcome::ChannelStruck(id) => {
            return format!(
                "{} {}.",
                tr(
                    "You channeled a strike against",
                    "Kamu menyalurkan serangan kepada"
                ),
                actor_name(locale, world, id)
            );
        }
        Outcome::MartialDrilled { teacher, learned } => {
            return format!(
                "{} {}{}.",
                tr("You drilled with", "Kamu berlatih bersama"),
                actor_name(locale, world, teacher),
                if learned {
                    tr(" and learned a form", " dan menguasai jurus")
                } else {
                    ""
                }
            );
        }
        Outcome::TechniqueStruck { target, form } => {
            return format!(
                "{form:?}: {} {}.",
                tr("you struck", "kamu menyerang"),
                actor_name(locale, world, target)
            );
        }
        Outcome::QiDrawn(qi) => return format!("Qi {qi} (one herb consumed)."),
        Outcome::QiCirculated { channels, blockage } => {
            return format!("Channels {channels}; blockage {blockage}.");
        }
        Outcome::MeridiansMended => ("Your meridians were treated.", "Jalur meridianmu dirawat."),
        Outcome::WoundTreated => (
            "One wound was treated using shrine medicine.",
            "Satu luka dirawat memakai obat kuil.",
        ),
        Outcome::HallAllocationClaimed => (
            "You claimed one physical herb from Reed Hall stock.",
            "Kamu mengambil satu herba fisik dari stok Balai Alang-alang.",
        ),
        Outcome::LotSold { to, id, coins } => {
            return format!(
                "{} #{id} {} {} {coins} {}.",
                tr("You sold lot", "Kamu menjual barang"),
                tr("to", "kepada"),
                actor_name(locale, world, to),
                tr("coins", "koin")
            );
        }
        Outcome::QiCentered { total } => return format!("Qi {total}/{MAX_QI}."),
        Outcome::FormStruck(id) => {
            return format!(
                "{} {}.",
                tr(
                    "You used Reed Strike on",
                    "Kamu memakai Jurus Alang-alang pada"
                ),
                actor_name(locale, world, id)
            );
        }
        Outcome::Delivered | Outcome::Contribution { witnesses: 0 } => (
            "You supplied the shrine with an herb.",
            "Kamu menyumbang satu herba ke kuil.",
        ),
        Outcome::Contribution { witnesses } => {
            return format!(
                "{} {witnesses}.",
                tr(
                    "You supplied a herb during a shortage. People who noticed",
                    "Kamu menyumbang herba saat stok kurang. Jumlah yang melihat"
                )
            );
        }
        Outcome::Practiced { progress } => {
            return format!(
                "{} {progress}/{BREATH_MASTERY}.",
                tr(
                    "You spent one herb on breath practice. Progress",
                    "Kamu memakai satu herba untuk latihan napas. Kemajuan"
                )
            );
        }
        Outcome::Struck(id) => {
            return format!(
                "{} {}.",
                tr("You struck", "Kamu menyerang"),
                actor_name(locale, world, id)
            );
        }
        Outcome::Guarded => ("You guarded this turn.", "Kamu bertahan pada giliran ini."),
        Outcome::BackgroundChosen(background) => {
            return background_label(locale, background).into();
        }
        Outcome::Traded { bought, coins } => {
            return format!(
                "{} {coins}.",
                if bought {
                    tr("You bought one herb for", "Kamu membeli satu herba seharga")
                } else {
                    tr("You sold one herb for", "Kamu menjual satu herba seharga")
                }
            );
        }
        Outcome::Enrolled => (
            "You became a Reed Hall apprentice.",
            "Kamu resmi menjadi murid Balai Alang-alang.",
        ),
        Outcome::Brewed => (
            "You brewed a remedy from two herbs.",
            "Kamu meracik satu obat dari dua herba.",
        ),
        Outcome::UsedRemedy { healed } => {
            return format!(
                "{} (+{healed} HP).",
                tr("You took a remedy", "Kamu meminum obat")
            );
        }
        Outcome::Treated { by, healed } => {
            return format!(
                "{} {} (+{healed} HP).",
                actor_name(locale, world, by),
                tr("treated you", "merawatmu")
            );
        }
        Outcome::Rejected(reason) => match reason {
            Rejection::Dead => (
                "You are dead. You can still wait.",
                "Kamu gugur. Kamu masih bisa menunggu.",
            ),
            Rejection::Impassable => ("The way is blocked.", "Jalan tertutup."),
            Rejection::Occupied => ("Someone is in the way.", "Seseorang menghalangi jalan."),
            Rejection::UnknownActor => ("That NPC is unavailable.", "NPC itu tidak tersedia."),
            Rejection::OutOfReach => (
                "You need to be within one tile.",
                "Kamu harus berada dalam jarak satu petak.",
            ),
            Rejection::Refused => (
                "The NPC refuses to help you.",
                "NPC itu menolak membantumu.",
            ),
            Rejection::HealerAway => ("Ask the healer at the clinic.", "Temui tabib di klinik."),
            Rejection::Empty => ("The supply is empty.", "Stok habis."),
            Rejection::NoEvidence => (
                "Nothing here records an institutional finding.",
                "Tidak ada temuan lembaga di petak ini.",
            ),
            Rejection::Overtrained => (
                "Too strained; wait and recover.",
                "Terlalu lelah; tunggu dan pulihkan diri.",
            ),
            Rejection::MeridiansBlocked => (
                "Blocked meridians require a healer.",
                "Meridian tersumbat perlu perawatan tabib.",
            ),
            Rejection::Hobbled => (
                "Your leg wound slows you this turn.",
                "Luka kaki memperlambat langkahmu giliran ini.",
            ),
            Rejection::NoCargo => ("You need more herbs.", "Herba yang kamu bawa belum cukup."),
            Rejection::PracticeComplete => (
                "You already mastered this practice.",
                "Kamu sudah menguasai latihan ini.",
            ),
            Rejection::BackgroundLocked => (
                "Choose a background before your first turn.",
                "Pilih latar sebelum giliran pertama.",
            ),
            Rejection::MarketClosed => ("The market opens at dawn.", "Pasar buka saat fajar."),
            Rejection::NotEnoughCoins => (
                "There are insufficient coins for the trade.",
                "Koin untuk transaksi ini kurang.",
            ),
            Rejection::HallClosed => (
                "Come back when the mentor is at the hall.",
                "Kembali saat guru berada di balai.",
            ),
            Rejection::AlreadyEnrolled => (
                "You are already an apprentice.",
                "Kamu sudah menjadi murid.",
            ),
            Rejection::PracticeRequired => (
                "Practice breath at the Qinghe shrine first.",
                "Berlatih napas di kuil Qinghe dulu.",
            ),
            Rejection::MembershipRequired => {
                ("Join Reed Hall first.", "Gabung Balai Alang-alang dulu.")
            }
            Rejection::NoRemedy => ("You have no remedies.", "Kamu tidak membawa obat."),
            Rejection::Uninjured => (
                "Save your remedy for a wound.",
                "Simpan obat untuk saat terluka.",
            ),
            Rejection::DutyComplete => (
                "You already reported the shrine duty.",
                "Tugas kuil sudah kamu laporkan.",
            ),
            Rejection::RankRequired => {
                if world.sect_hierarchy().is_some() && world.hall_rank() == Some(HallRank::Disciple)
                {
                    (
                        "Earn more merit to reach senior disciple rank first.",
                        "Raih lebih banyak jasa untuk mencapai pangkat murid senior dulu.",
                    )
                } else {
                    (
                        "Complete your shrine duty and become a disciple first.",
                        "Tuntaskan tugas kuil dan jadilah murid dalam dulu.",
                    )
                }
            }
            Rejection::FormAlreadyKnown => (
                "You already know Reed Strike.",
                "Kamu sudah menguasai Jurus Alang-alang.",
            ),
            Rejection::FormRequired => (
                "Learn Reed Strike at the hall first.",
                "Pelajari Jurus Alang-alang di balai dulu.",
            ),
            Rejection::QiFull => ("Your Qi is full.", "Qi kamu sudah penuh."),
            Rejection::IntroductionRequired => (
                "Talk to each other first.",
                "Berkenalanlah dulu lewat talk.",
            ),
            Rejection::BoundToDuty => (
                "This person stays with their duties.",
                "Orang ini memilih tetap bertugas.",
            ),
            Rejection::CompanyFull => (
                "You already have a travelling companion.",
                "Kamu sudah memiliki pendamping.",
            ),
            Rejection::NotCompanion => (
                "You are not travelling together.",
                "Kalian tidak sedang bepergian bersama.",
            ),
            Rejection::PackFull => (
                "The medicine pouch holds two herbs and is full.",
                "Kantung obat berisi dua herba dan sudah penuh.",
            ),
            Rejection::SchoolContested => (
                "Stonebank's claim has suspended the teacher's service. Negotiate an accord or sponsor Reed Hall.",
                "Tuntutan perguruan Batu Tepi menghentikan layanan guru. Negosiasikan accord atau sponsor Balai Alang-alang.",
            ),
            Rejection::OathConflict => (
                "Your current allegiance prevents that action.",
                "Kesetiaanmu saat ini menghalangi tindakan itu.",
            ),
            Rejection::FactionTrustRequired => (
                "Earn the faction's trust through material support first.",
                "Bangun kepercayaan perguruan melalui dukungan nyata terlebih dahulu.",
            ),
            Rejection::InsufficientQi => {
                return format!(
                    "{} {FORM_QI_COST} Qi.",
                    tr("Reed Strike needs", "Jurus Alang-alang membutuhkan")
                );
            }
        },
    };
    text(locale, en, id).into()
}

pub(crate) fn run(debug_world: bool) {
    run_campaign(debug_world, 26);
}

fn hall_rank_label(locale: Locale, rank: HallRank) -> &'static str {
    let (en, id) = match rank {
        HallRank::Apprentice => ("Reed Hall apprentice", "Murid pemula Balai Alang-alang"),
        HallRank::Disciple => ("Reed Hall disciple", "Murid dalam Balai Alang-alang"),
        HallRank::SeniorDisciple => (
            "Reed Hall senior disciple",
            "Murid senior Balai Alang-alang",
        ),
        HallRank::Elder => ("Reed Hall elder", "Tetua Balai Alang-alang"),
    };
    text(locale, en, id)
}

fn decision_reason_label(locale: Locale, reason: DecisionReason) -> &'static str {
    let (en, id) = match reason {
        DecisionReason::ImmediateDanger => ("immediate danger", "bahaya langsung"),
        DecisionReason::PhysicalRecovery => ("physical recovery", "pemulihan tubuh"),
        DecisionReason::TreatmentFunds => ("treatment costs", "biaya perawatan"),
        DecisionReason::MedicalDuty => ("medical duty", "tugas pengobatan"),
        DecisionReason::Cultivation => ("cultivation", "kultivasi"),
        DecisionReason::MartialAmbition => ("martial ambition", "ambisi bela diri"),
        DecisionReason::InstitutionalObligation => {
            ("institutional obligation", "kewajiban kelembagaan")
        }
        DecisionReason::KnownOpportunity => ("a known opportunity", "peluang yang diketahui"),
        DecisionReason::Fatigue => ("fatigue", "kelelahan"),
        DecisionReason::Livelihood => ("livelihood", "mata pencaharian"),
        DecisionReason::Routine => ("routine", "rutinitas"),
        DecisionReason::ResumingAmbition => (
            "resuming an interrupted pursuit",
            "melanjutkan tujuan yang terputus",
        ),
    };
    text(locale, en, id)
}

pub(crate) fn run_c35() {
    run_campaign(false, 25);
}

pub(crate) fn run_c34() {
    run_campaign(false, 24);
}

pub(crate) fn run_c33() {
    run_campaign(false, 23);
}

pub(crate) fn run_c32() {
    run_campaign(false, 22);
}

pub(crate) fn run_c31() {
    run_campaign(false, 21);
}

pub(crate) fn run_c30() {
    run_campaign(false, 20);
}

pub(crate) fn run_c29() {
    run_campaign(false, 19);
}

pub(crate) fn run_c28() {
    run_campaign(false, 18);
}

pub(crate) fn run_c27() {
    run_campaign(false, 17);
}

pub(crate) fn run_c25() {
    run_campaign(false, 16);
}

pub(crate) fn run_c24() {
    run_campaign(false, 15);
}

pub(crate) fn run_c23() {
    run_campaign(false, 14);
}

pub(crate) fn run_c22() {
    run_campaign(false, 13);
}

pub(crate) fn run_c21() {
    run_campaign(false, 12);
}

pub(crate) fn run_c14() {
    run_campaign(false, 5);
}

pub(crate) fn run_c15() {
    run_campaign(false, 6);
}

pub(crate) fn run_c16() {
    run_campaign(false, 7);
}

pub(crate) fn run_c17() {
    run_campaign(false, 8);
}

pub(crate) fn run_c18() {
    run_campaign(false, 9);
}

pub(crate) fn run_c19() {
    run_campaign(false, 10);
}

pub(crate) fn run_c20() {
    run_campaign(false, 11);
}

pub(crate) fn campaign_session(debug_world: bool, revision: u16) -> Session {
    let path = persistence::default_path().with_file_name(match revision {
        5 => "checkpoint.c10",
        6 => "checkpoint.c15",
        7 => "checkpoint.c16",
        8 => "checkpoint.c17",
        9 => "checkpoint.c18",
        10 => "checkpoint.c19",
        11 => "checkpoint.c20",
        12 => "checkpoint.c21",
        13 => "checkpoint.c22",
        14 => "checkpoint.c23",
        15 => "checkpoint.c24",
        16 => "checkpoint.c25",
        17 => "checkpoint.c27",
        18 => "checkpoint.c28",
        19 => "checkpoint.c29",
        20 => "checkpoint.c30",
        21 => "checkpoint.c31",
        22 => "checkpoint.c32",
        23 => "checkpoint.c33",
        24 => "checkpoint.c34",
        25 => "checkpoint.c35",
        26 => "checkpoint.c351",
        _ => unreachable!("unsupported campaign revision"),
    });
    let mut session = Session::new(path);
    session.world = match revision {
        5 => World::road_company(0),
        6 => World::sect_rivalry(0),
        7 => World::sect_politics(0),
        8 => World::traffic_courtesy(0),
        9 => World::living_agency(0),
        10 => World::faction_agency(0),
        11 => World::institutional_agency(0),
        12 => World::contract_accountability(0),
        13 => World::information_disputes(0),
        14 => World::player_agency(0),
        15 => World::consequences(0),
        16 => World::violence_logistics(0),
        17 => World::player_reporting(0),
        18 => World::physical_recovery(0),
        19 => World::contested_resources(0),
        20 => World::sect_cultivation(0),
        21 => World::martial_mentorship(0),
        22 => World::meridian_cultivation(0),
        23 => World::persistent_injury(0),
        24 => World::autonomous_agency(0),
        25 => World::sect_hierarchy_campaign(0),
        26 => World::duty_coherence_campaign(0),
        _ => unreachable!("unsupported campaign revision"),
    };
    session.debug_world = debug_world;
    session
}

fn run_campaign(debug_world: bool, revision: u16) {
    let mut session = campaign_session(debug_world, revision);
    if let Ok(script) = std::env::var("SEALED_ASCII_COMMANDS") {
        println!("{}", session.frame());
        // Semicolons support explicit IDs. A comma or whitespace sequence of
        // single-token commands still works with existing smoke inputs.
        if script.contains(';') {
            for command in script.split(';') {
                if !session.input(command) {
                    break;
                }
                println!("{}", session.frame());
            }
        } else {
            for command in script
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
            {
                if !session.input(command) {
                    break;
                }
                println!("{}", session.frame());
            }
        }
        return;
    }

    let stdin = io::stdin();
    let mut input = stdin.lock().lines();
    loop {
        println!("{}", session.frame());
        print!("> ");
        let _ = io::stdout().flush();
        if !input
            .next()
            .is_some_and(|line| line.is_ok_and(|line| session.input(&line)))
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_keeps_existing_foreign_and_corrupt_saves_intact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint.c35");
        let mut session = Session::new(path.clone());
        session.world = World::sect_hierarchy_campaign(0);
        session.input("wait");
        let before = session.world.checkpoint();
        let foreign = World::institutional_agency(0).checkpoint();
        for existing in [foreign, b"corrupt".to_vec()] {
            std::fs::write(&path, &existing).unwrap();
            session.input("load");
            assert!(session.notice.contains("Load failed"));
            assert_eq!(session.world.checkpoint(), before);
            session.input("save");
            assert!(session.notice.contains("Save failed"));
            assert_eq!(std::fs::read(&path).unwrap(), existing);
        }
        std::fs::remove_file(&path).unwrap();
        session.input("save");
        assert!(session.notice.contains("Saved the settlement"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        session.input("wait");
        session.input("save");
        let current = session.world.checkpoint();
        session.input("wait");
        session.input("load");
        assert!(session.notice.contains("Restored the settlement"));
        assert_eq!(session.world.checkpoint(), current);
    }

    #[test]
    fn recovery_current_campaign_has_the_current_save_slot() {
        let session = campaign_session(false, 25);
        assert_eq!(session.world.checkpoint_revision(), 25);
        assert_eq!(session.save_path.file_name().unwrap(), "checkpoint.c35");
    }

    #[test]
    fn recovery_revision26_isolates_save_slot_and_preserves_revision_on_reset() {
        let mut session = campaign_session(false, 26);
        assert_eq!(session.world.checkpoint_revision(), 26);
        assert_eq!(session.save_path.file_name().unwrap(), "checkpoint.c351");
        session.input("seed 7");
        assert_eq!(session.world.checkpoint_revision(), 26);
        assert_eq!(session.world.region().unwrap().seed, 7);
        session.input("wait");
        session.input("reset");
        assert_eq!(session.world.checkpoint_revision(), 26);
        assert_eq!(session.world.region().unwrap().seed, 7);
        assert_eq!(session.world.tick(), 0);
        let legacy = campaign_session(false, 25);
        assert_eq!(legacy.save_path.file_name().unwrap(), "checkpoint.c35");
        assert_eq!(legacy.world.checkpoint_revision(), 25);
        let dir = tempfile::tempdir().unwrap();
        session.save_path = dir.path().join("checkpoint.c351");
        let foreign = legacy.world.checkpoint();
        std::fs::write(&session.save_path, &foreign).unwrap();
        let before = session.world.checkpoint();
        session.input("load");
        assert!(session.notice.contains("Load failed"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("save");
        assert!(session.notice.contains("Save failed"));
        assert_eq!(std::fs::read(&session.save_path).unwrap(), foreign);
    }

    #[test]
    fn recovery_rejects_lossy_terminal_commands_without_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint.c35");
        let mut session = Session::new(path.clone());
        session.world = World::sect_hierarchy_campaign(0);
        let before = session.world.checkpoint();
        for command in ["tell 257 0", "tell 1 16777216"] {
            session.input(command);
            assert_eq!(session.world.checkpoint(), before);
        }
        session.input("save");
        assert_eq!(std::fs::read(&path).unwrap(), before);
        session.input("load");
        assert!(session.notice.contains("Restored the settlement"));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn the_terminal_renders_canonical_ticks_activity_and_restored_stock() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("checkpoint.c10"));
        session.input("look");
        assert_eq!(session.world.tick(), 0);
        assert!(session.frame().contains("Sealed Heaven - Qinghe"));
        session.input("d");
        session.input("wait");
        assert_eq!(session.world.player().position, Position { x: 1, y: 4 });
        assert!(session.frame().contains("at the clinic"));
        assert!(session.frame().contains("Shrine stock"));
        let saved = session.world.checkpoint();
        session.input("save");
        session.input("n");
        assert_ne!(session.world.checkpoint(), saved);
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        assert!(session.frame().contains("Restored the settlement."));
    }

    #[test]
    fn normal_view_hides_private_goals_remote_people_and_remote_stock() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.input("wait");
        let normal = session.frame();
        assert!(normal.contains("Shen Wei"));
        assert!(!normal.contains("patrolling"));
        assert!(!normal.contains("Lin Su"));
        assert!(!normal.contains("Road outrider"));
        assert!(!normal.contains("Herbs in patch"));
        assert!(!normal.contains("NPC deliveries"));

        let before = session.world.checkpoint();
        session.input("attack 22");
        let hidden = session.notice.clone();
        session.input("attack 999");
        assert_eq!(hidden, session.notice);
        session.input("inspect 1");
        assert!(session.notice.contains("Yao Lian (healer)"));
        session.input("inspect 22");
        assert_eq!(session.notice, hidden);
        assert_eq!(session.world.checkpoint(), before);

        session.debug_world = true;
        let developer = session.frame();
        assert!(developer.contains("DEVELOPER VIEW"));
        assert!(developer.contains("patrolling"));
        assert!(developer.contains("collecting herbs"));
        assert!(developer.contains("Road outrider"));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn changing_language_changes_the_ui_and_keeps_checkpoint_bytes() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.input("wait");
        let before = session.world.checkpoint();
        session.input("lang id");
        assert!(session.frame().contains("Giliran 1"));
        assert!(session.frame().contains("Waktu berlalu."));
        assert!(!session.frame().contains("Time passed."));
        assert_eq!(session.world.checkpoint(), before);
        session.input("lang en");
        assert!(session.frame().contains("Time passed."));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn regional_seed_and_background_enter_the_save_through_the_terminal() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.input("seed 1");
        assert_eq!(session.world.tick(), 0);
        session.input("background herbalist");
        let before = session.world.checkpoint();
        assert_eq!(
            session.world.region().unwrap().background,
            Some(Background::Herbalist)
        );
        assert!(!session.frame().contains("Market herbs"));
        session.input("seed 2");
        assert_eq!(session.world.checkpoint(), before);
        assert_eq!(World::restore(&before).unwrap(), session.world);
    }

    #[test]
    fn terminal_keeps_school_actions_bound_to_the_school_campaign() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::riverlands(0);
        let before = session.world.checkpoint();
        session.input("duty");
        assert_eq!(session.world.checkpoint(), before);
        assert!(session.notice.contains("requires a new Reed Hall campaign"));
        session.input("reset");
        assert!(session.world.school().is_some());
        session.input("lang id");
        session.input("center");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::FormRequired))
        );
        assert!(session.frame().contains("Pelajari Jurus Alang-alang"));
        assert!(!session.frame().contains("Luo Xiu"));
    }

    #[test]
    fn journal_and_hidden_companion_targets_do_not_advance_time_or_disclose_remote_people() {
        let mut session = Session::new(PathBuf::from("unused"));
        let bytes = session.world.checkpoint();
        for cmd in ["invite 22", "pack 22", "part 22"] {
            session.input(cmd);
            assert_eq!(session.notice, session.target_hint());
            assert!(!session.frame().contains("Road outrider"));
        }
        session.input("journal");
        assert!(session.notice.contains("No recorded"));
        session.input("lang id");
        session.input("journal");
        assert!(session.notice.contains("Belum ada catatan"));
        assert_eq!(session.world.checkpoint(), bytes);
    }

    #[test]
    fn bad_and_legacy_saves_leave_the_live_world_intact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint.c10");
        let mut session = Session::new(path.clone());
        session.input("n");
        let before = session.world.checkpoint();
        std::fs::write(
            &path,
            sealed_sim::player5::World::conflict_with_memory().checkpoint(),
        )
        .unwrap();
        session.input("load");
        assert!(session.frame().contains("--c09"));
        assert_eq!(session.world.checkpoint(), before);
        std::fs::write(path, b"bad").unwrap();
        session.input("load");
        assert!(session.frame().contains("Load failed"));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn rivalry_journal_obeys_line_of_sight_and_commands_preserve_read_only_time() {
        let mut session = Session::new(PathBuf::from("unused"));
        assert!(session.world.rivalry().is_some());
        let before = session.world.checkpoint();
        session.input("factions");
        assert!(session.notice.contains("Ask in Willowford"));
        session.input("factionlog");
        assert!(session.notice.contains("You witnessed no sect events"));
        session.input("lang id");
        session.input("factions");
        assert!(session.notice.contains("Cari kabar di Willowford"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("accord");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::OutOfReach))
        );
    }

    #[test]
    fn legacy_campaign_reset_and_seed_do_not_silently_upgrade_to_revision_six() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::road_company(0);
        session.input("seed 1");
        assert!(session.world.rivalry().is_none());
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            5
        );
        session.input("reset");
        assert!(session.world.rivalry().is_none());
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            5
        );
        session.input("accord");
        assert!(session.notice.contains("C1.5 campaign"));
        assert_eq!(session.world.tick(), 0);
    }

    #[test]
    fn political_actions_and_journal_are_localized_without_leaking_unseen_events() {
        let mut session = Session::new(PathBuf::from("unused"));
        assert!(session.world.politics().is_some());
        let before = session.world.checkpoint();
        session.input("politicslog");
        assert!(session.notice.contains("No witnessed political events"));
        session.input("factions");
        assert!(session.notice.contains("Ask in Willowford"));
        session.input("lang id");
        session.input("politicslog");
        assert!(session.notice.contains("Belum ada peristiwa politik"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("pledge");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::OutOfReach))
        );
    }

    #[test]
    fn revision_six_client_seed_and_reset_keep_campaign_rules() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::sect_rivalry(0);
        session.input("seed 1");
        assert!(session.world.rivalry().is_some());
        assert!(session.world.politics().is_none());
        session.input("reset");
        assert!(session.world.politics().is_none());
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            6
        );
        session.input("pledge");
        assert!(session.notice.contains("C1.6 campaign"));
        assert_eq!(session.world.tick(), 0);
    }

    #[test]
    fn traffic_log_is_private_localized_and_read_only() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::traffic_courtesy(0);
        assert!(session.world.traffic().is_some());
        let before = session.world.checkpoint();
        session.input("trafficlog");
        assert!(session.notice.contains("No witnessed passage"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("lang id");
        session.input("trafficlog");
        assert!(session.notice.contains("Belum ada pertukaran"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            8
        );
        session.input("reset");
        assert!(session.world.traffic().is_some());
    }

    #[test]
    fn c16_campaign_seed_and_reset_remain_revision_seven() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::sect_politics(0);
        session.input("seed 1");
        assert!(session.world.traffic().is_none());
        session.input("reset");
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            7
        );
        session.input("trafficlog");
        assert!(session.notice.contains("No witnessed passage"));
    }

    #[test]
    fn player_exchange_appears_in_visible_terminal_traffic_log() {
        let mut session = Session::new(PathBuf::from("unused"));
        for command in ["d", "d", "d", "d", "s"] {
            session.input(command);
        }
        assert_eq!(session.world.traffic().unwrap().swaps, 1);
        let turn = session.world.tick();
        session.input("trafficlog");
        assert!(session.notice.contains("exchanged places with Shen Wei"));
        assert_eq!(session.world.tick(), turn);
        session.input("lang id");
        session.input("trafficlog");
        assert!(session.notice.contains("bertukar tempat dengan Shen Wei"));
        assert_eq!(session.world.tick(), turn);
    }

    #[test]
    fn c18_relief_log_is_localized_and_inspection_never_advances_time() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::living_agency(0);
        assert!(session.world.relief().is_some());
        let before = session.world.checkpoint();
        session.input("relieflog");
        assert!(session.notice.contains("No witnessed relief events"));
        session.input("lang id");
        session.input("relieflog");
        assert!(session.notice.contains("Belum ada kejadian bantuan"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            9
        );
        session.input("reset");
        assert!(session.world.relief().is_some());
        assert!(session.world.faction_agency_state().is_none());
    }

    #[test]
    fn c19_aidlog_is_localized_read_only_and_hides_unseen_reconnaissance() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::faction_agency(0);
        assert!(session.world.faction_agency_state().is_some());
        let before = session.world.checkpoint();
        session.input("aidlog");
        assert!(session.notice.contains("No witnessed faction aid events"));
        session.input("lang id");
        session.input("aidlog");
        assert!(session.notice.contains("Belum ada bantuan antarfaksi"));
        assert_eq!(session.world.checkpoint(), before);
        for _ in 0..30 {
            session.input("wait");
        }
        assert_eq!(session.world.faction_agency_state().unwrap().surveys, 1);
        // A distant NPC's private journey is not visible to the player.
        session.input("aidlog");
        assert!(session.notice.contains("Belum ada bantuan antarfaksi"));
    }

    #[test]
    fn c19_seed_reset_and_checkpoint_do_not_mutate_revision_nine_saves() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::faction_agency(0);
        session.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            10
        );
        session.input("reset");
        assert!(session.world.faction_agency_state().is_some());
        assert_eq!(session.world.region().unwrap().seed, 1);
        let mut legacy = Session::new(PathBuf::from("unused"));
        legacy.world = World::living_agency(1);
        legacy.input("seed 0");
        assert!(legacy.world.faction_agency_state().is_none());
        assert_eq!(
            u16::from_le_bytes(legacy.world.checkpoint()[12..14].try_into().unwrap()),
            9
        );
        legacy.input("reset");
        assert!(legacy.world.faction_agency_state().is_none());
    }

    #[test]
    fn c20_contractlog_is_read_only_and_localized() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::institutional_agency(0);
        assert!(session.world.contracts().is_some());
        let checkpoint = session.world.checkpoint();
        session.input("contractlog");
        assert!(
            session
                .notice
                .contains("No witnessed institutional contracts")
        );
        session.input("lang id");
        session.input("contractlog");
        assert!(session.notice.contains("Belum ada kontrak lembaga"));
        assert_eq!(session.world.checkpoint(), checkpoint);
    }

    #[test]
    fn c20_seed_reset_and_legacy_c19_campaigns_stay_isolated() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::institutional_agency(0);
        session.input("seed 1");
        assert_eq!(session.world.region().unwrap().seed, 1);
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            11
        );
        session.input("reset");
        assert!(session.world.contracts().is_some());
        let mut previous = Session::new(PathBuf::from("unused"));
        previous.world = World::faction_agency(0);
        previous.input("seed 1");
        previous.input("reset");
        assert!(previous.world.contracts().is_none());
        assert!(previous.world.faction_agency_state().is_some());
        assert_eq!(
            u16::from_le_bytes(previous.world.checkpoint()[12..14].try_into().unwrap()),
            10
        );
        previous.input("contractlog");
        assert!(
            previous
                .notice
                .contains("No witnessed institutional contracts")
        );
    }

    #[test]
    fn c21_accountlog_is_witness_only_localized_and_read_only() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::contract_accountability(0);
        assert!(session.world.contract_memory().is_some());
        assert!(session.world.information().is_none());
        let before = session.world.checkpoint();
        session.input("accountlog");
        assert!(session.notice.contains("No witnessed contract audits"));
        session.input("lang id");
        session.input("accountlog");
        assert!(session.notice.contains("Belum ada pemeriksaan kontrak"));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn c22_infolog_is_witness_only_localized_and_read_only() {
        let mut session = Session::new(PathBuf::from("unused"));
        assert!(session.world.information().is_some());
        let before = session.world.checkpoint();
        session.input("infolog");
        assert!(
            session
                .notice
                .contains("No witnessed institutional information")
        );
        session.input("lang id");
        session.input("infolog");
        assert!(session.notice.contains("Belum ada informasi lembaga"));
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn c21_campaign_flag_stays_on_revision_twelve_without_information() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::contract_accountability(0);
        assert!(session.world.information().is_none());
        assert!(session.world.contract_memory().is_some());
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            12
        );
        session.input("help");
        assert!(session.notice.contains("C2.1"));
        assert!(!session.notice.contains("C2.2"));
        session.input("infolog");
        assert!(
            session
                .notice
                .contains("No witnessed institutional information")
        );
        assert!(session.world.information().is_none());
        session.input("accountlog");
        assert!(session.notice.contains("No witnessed contract audits"));
        let before = session.world.checkpoint();
        session.input("lang id");
        session.input("infolog");
        assert_eq!(
            &session.world.checkpoint()[..before.len().min(session.world.checkpoint().len())],
            &before[..before.len().min(session.world.checkpoint().len())]
        );
        assert_eq!(session.world.checkpoint(), before);
    }

    #[test]
    fn c21_seeding_reset_and_c20_legacy_keep_distinct_save_revisions() {
        let mut current = Session::new(PathBuf::from("unused"));
        current.world = World::contract_accountability(0);
        current.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(current.world.checkpoint()[12..14].try_into().unwrap()),
            12
        );
        current.input("reset");
        assert!(current.world.contract_memory().is_some());
        assert!(current.world.information().is_none());
        assert_eq!(current.world.region().unwrap().seed, 1);
        let mut previous = Session::new(PathBuf::from("unused"));
        previous.world = World::institutional_agency(0);
        previous.input("seed 1");
        previous.input("reset");
        assert!(previous.world.contracts().is_some());
        assert!(previous.world.contract_memory().is_none());
        assert_eq!(
            u16::from_le_bytes(previous.world.checkpoint()[12..14].try_into().unwrap()),
            11
        );
        previous.input("accountlog");
        assert!(previous.notice.contains("No witnessed contract audits"));
    }

    #[test]
    fn c22_seeding_reset_and_c21_legacy_keep_distinct_save_revisions() {
        let mut current = Session::new(PathBuf::from("unused"));
        current.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(current.world.checkpoint()[12..14].try_into().unwrap()),
            13
        );
        current.input("reset");
        assert!(current.world.information().is_some());
        assert!(current.world.contract_memory().is_some());
        assert_eq!(current.world.region().unwrap().seed, 1);
        let mut previous = Session::new(PathBuf::from("unused"));
        previous.world = World::contract_accountability(0);
        previous.input("seed 1");
        previous.input("reset");
        assert!(previous.world.contract_memory().is_some());
        assert!(previous.world.information().is_none());
        assert_eq!(
            u16::from_le_bytes(previous.world.checkpoint()[12..14].try_into().unwrap()),
            12
        );
        previous.input("infolog");
        assert!(
            previous
                .notice
                .contains("No witnessed institutional information")
        );
    }

    #[test]
    fn c17_seed_and_reset_do_not_silently_upgrade_to_relief_campaign() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::traffic_courtesy(0);
        session.input("seed 1");
        assert!(session.world.relief().is_none());
        session.input("reset");
        assert!(session.world.relief().is_none());
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            8
        );
    }

    #[test]
    fn c23_know_is_read_only_in_both_languages() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::player_agency(0);
        let before = session.world.checkpoint();
        session.input("know");
        assert!(session.notice.contains("You know no institutional finding"));
        session.input("lang id");
        session.input("know");
        assert!(session.notice.contains("belum mengetahui temuan lembaga"));
        session.input("help");
        assert!(session.notice.contains("C2.3"));
        assert_eq!(session.world.checkpoint(), before);
        assert_eq!(session.world.tick(), 0);
    }

    #[test]
    fn c23_public_commands_examine_and_can_withhold() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::player_agency(0);
        let tick = session.world.tick();
        session.input("examine");
        let view = session.frame();
        assert!(
            view.contains("supply is empty") || view.contains("examined evidence"),
            "frame was: {view}"
        );
        assert!(session.world.tick() > tick);
        session.input("tell 33 0");
        let view = session.frame();
        assert!(
            view.contains("within one tile")
                || view.contains("refuses")
                || view.contains("unavailable")
                || view.contains("offered finding"),
            "frame was: {view}"
        );
        let after = session.world.checkpoint();
        session.input("know");
        session.input("lang id");
        session.input("know");
        assert!(session.notice.contains("belum mengetahui"));
        assert_eq!(session.world.checkpoint(), after);
    }

    #[test]
    fn c24_empty_ground_and_a_living_target_are_distinct() {
        let mut session = Session::new(PathBuf::from("unused"));
        session.world = World::consequences(0);
        session.input("examine");
        assert!(session.frame().contains("institutional finding"));
        session.input("lang id");
        session.input("examine");
        assert!(session.frame().contains("temuan lembaga"));
        session.input("lang en");
        session.input("help");
        assert!(session.notice.contains("C2.4"));
        let tick = session.world.tick();
        session.input("take 2");
        assert!(session.notice.contains("visible NPC"));
        assert_eq!(session.world.tick(), tick);
        session.input("know");
        assert_eq!(session.world.tick(), tick);
    }

    #[test]
    fn c25_ascii_road_ambush_loot_and_save_load_use_only_public_commands() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("road.c25"));
        session.world = World::violence_logistics(0);
        for _ in 0..14 {
            session.input("d");
            assert_eq!(session.world.latest_outcome(), Some(Outcome::Moved));
        }
        for command in [
            "wait",
            "attack 32",
            "w",
            "d",
            "d",
            "w",
            "w",
            "d",
            "d",
            "s",
            "attack 32",
            "attack 32",
        ] {
            session.input(command);
            assert!(
                !matches!(session.world.latest_outcome(), Some(Outcome::Rejected(_))),
                "{command}: {}",
                session.notice
            );
        }
        assert_eq!(session.world.tick(), 26);
        assert!(!session.world.npc(ActorId::new(32)).unwrap().alive());
        assert_eq!(session.world.npc(ActorId::new(32)).unwrap().herbs, 1);
        let tick = session.world.tick();
        session.input("bodies");
        assert!(session.notice.contains("Bao Jun"));
        assert_eq!(session.world.tick(), tick);
        session.input("take 32");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Took {
                from: ActorId::new(32),
                herbs: 1,
            })
        );
        let view = session.frame();
        assert!(view.contains("t27 You took cargo from Bao Jun"));
        assert!(!view.contains("t27 You hit Bao Jun (0 HP)"));
        let before = session.world.checkpoint();
        session.input("crimelog");
        assert!(
            session
                .notice
                .contains("t26 You hit Bao Jun (you did this)")
        );
        assert!(
            session
                .notice
                .contains("t27 You took cargo from Bao Jun (you did this)")
        );
        session.input("lang id");
        session.input("crimelog");
        assert!(session.notice.contains("kamu melakukannya"));
        assert_eq!(session.world.checkpoint(), before);
        session.input("lang en");
        assert_eq!(session.world.npc(ActorId::new(32)).unwrap().herbs, 0);
        session.input("save");
        let saved = session.world.checkpoint();
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        session.input("seed 1");
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            16
        );
        session.input("help");
        assert!(session.notice.contains("C2.5"));
        assert_eq!(session.world.checkpoint(), saved);
    }

    #[test]
    fn c26_investigation_views_do_not_tick_or_upgrade_old_campaigns() {
        let mut current = Session::new(PathBuf::from("unused"));
        current.world = World::violence_logistics(1);
        let before = current.world.checkpoint();
        current.input("crimelog");
        assert!(current.notice.contains("You know no assaults or thefts"));
        current.input("bodies");
        assert!(current.notice.contains("You have discovered no bodies"));
        assert_eq!(current.world.checkpoint(), before);
        let mut old = Session::new(PathBuf::from("unused"));
        old.world = World::consequences(1);
        let before = old.world.checkpoint();
        old.input("crimelog");
        assert!(old.notice.contains("Only in C2.5"));
        assert_eq!(old.world.checkpoint(), before);
    }

    #[test]
    fn c27_ascii_corpse_report_reaches_watch_without_inventing_murder_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("report.c27"));
        session.world = World::player_reporting(0);
        let body = ActorId::new(32);
        let watch = ActorId::new(2);
        for _ in 0..14 {
            session.input("d");
        }
        for command in [
            "wait",
            "attack 32",
            "w",
            "d",
            "d",
            "w",
            "w",
            "d",
            "d",
            "s",
            "attack 32",
            "attack 32",
            "s",
        ] {
            session.input(command);
            assert!(
                !matches!(session.world.latest_outcome(), Some(Outcome::Rejected(_))),
                "{command}: {}",
                session.notice
            );
        }
        for _ in 0..8 {
            session.input("a");
        }
        for _ in 0..3 {
            session.input("wait");
        }
        assert_eq!(session.world.tick(), 38);
        session.input("reportbody 2 32");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::BodyReported { to: watch, body })
        );
        session.input("save");
        let saved = session.world.checkpoint();
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        session.input("wait");
        session.input("wait");
        let report = session
            .world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == body)
            .unwrap();
        assert!(
            matches!(report.source, KnowledgeSource::Reported { sender, hops: 1, .. }
            if sender == PLAYER)
        );
        assert_eq!(
            session
                .world
                .social()
                .knowledge(watch, sealed_sim::living::IncidentId(4)),
            None
        );
        session.input("wait");
        let confirmed = session
            .world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == body)
            .unwrap();
        assert_eq!(confirmed.source, KnowledgeSource::Observed);
        assert_eq!(
            confirmed.prior_report,
            Some((
                41,
                KnowledgeSource::Reported {
                    sender: PLAYER,
                    witness: PLAYER,
                    hops: 1
                }
            ))
        );
        session.input("help");
        assert!(session.notice.contains("C2.7"));
        let prior = session.world.checkpoint();
        session.input("reportbody 2");
        assert_eq!(session.world.checkpoint(), prior);
    }

    #[test]
    fn c28_ascii_witnesses_physical_recovery_and_preserves_save() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("recovery.c28"));
        session.world = World::physical_recovery(0);
        for _ in 0..14 {
            session.input("d");
        }
        for command in [
            "wait",
            "attack 32",
            "w",
            "d",
            "d",
            "w",
            "w",
            "d",
            "d",
            "s",
            "attack 32",
            "attack 32",
        ] {
            session.input(command);
            assert!(
                !matches!(session.world.latest_outcome(), Some(Outcome::Rejected(_))),
                "{command}: {}",
                session.notice
            );
        }
        for _ in 26..47 {
            session.input("wait");
        }
        assert_eq!(session.world.social().recoveries().len(), 1);
        session.input("salvagelog");
        assert!(session.notice.contains("recovered 1 herbs from Bao Jun"));
        let before = session.world.checkpoint();
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), before);
        for _ in 47..75 {
            session.input("wait");
        }
        assert_eq!(session.world.social().recoveries().len(), 2);
        assert_eq!(session.world.npc(ActorId::new(32)).unwrap().herbs, 0);
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            18
        );
        session.input("help");
        assert!(session.notice.contains("C2.8"));
        let saved = session.world.checkpoint();
        session.input("salvagelog");
        assert_eq!(session.world.checkpoint(), saved);
    }

    #[test]
    fn c29_ascii_claim_and_sale_compete_with_autonomous_sects_without_leaking_unseen_lots() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("contest.c29"));
        session.world = World::contested_resources(0);
        let pristine = session.world.checkpoint();
        session.input("lots");
        assert!(!session.notice.contains("#1"));
        assert!(!session.notice.contains("#2"));
        session.input("lotlog");
        assert!(session.notice.contains("No witnessed"));
        assert_eq!(session.world.checkpoint(), pristine);
        for _ in 0..8 {
            session.input("d");
        }
        session.input("lots");
        assert!(session.notice.contains("#0 SpiritOre"));
        session.input("claim 0");
        assert_eq!(
            session.world.contested().unwrap().lots[&0].holder,
            Holder::Carried(PLAYER)
        );
        assert_eq!(session.world.latest_outcome(), Some(Outcome::LotClaimed(0)));
        session.input("lotlog");
        assert!(session.notice.contains("lot #0"));
        assert!(!session.notice.contains("lot #1"));
        assert!(!session.notice.contains("lot #2"));
        let saved = session.world.checkpoint();
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        for _ in 0..20 {
            session.input("d");
        }
        session.input("s");
        session.input("offerlot 36 0");
        assert_eq!(
            session.world.contested().unwrap().lots[&0].holder,
            Holder::Carried(ActorId::new(36))
        );
        assert!(
            matches!(session.world.latest_outcome(), Some(Outcome::LotSold { to, id: 0, coins: 2 }) if to == ActorId::new(36))
        );
        let once = session.world.contested().unwrap().history.len();
        session.input("offerlot 36 0");
        assert!(matches!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(_))
        ));
        assert_eq!(session.world.contested().unwrap().history.len(), once);
        session.input("help");
        assert!(session.notice.contains("C2.9"));
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            19
        );
    }

    #[test]
    fn c30_ascii_relics_enable_wards_and_jade_channel_without_duplication() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("cultivation.c30"));
        session.world = World::sect_cultivation(0);
        for _ in 0..5 {
            session.input("d");
        }
        session.input("claim 3");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::LotClaimed(3)));
        session.input("attune 3");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::LotAttuned(3)));
        for _ in 0..3 {
            session.input("d");
        }
        session.input("claim 0");
        session.input("attune 0");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::LotAttuned(0)));
        let prior = session.world.checkpoint();
        session.input("cultivation");
        assert!(session.notice.contains("Wards: 2; Jade channels: 2"));
        session.input("lots");
        assert!(!session.notice.contains("#0"));
        assert!(!session.notice.contains("#3"));
        assert_eq!(session.world.checkpoint(), prior);
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), prior);
        session.input("channel 32");
        assert_eq!(session.world.contested().unwrap().script_charges, 2);
        session.input("wait");
        for _ in 0..3 {
            session.input("d");
        }
        session.input("wait");
        session.input("channel 32");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::ChannelStruck(ActorId::new(32)))
        );
        assert_eq!(session.world.contested().unwrap().script_charges, 1);
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            20
        );
        session.input("help");
        assert!(session.notice.contains("C3.0"));
    }

    #[test]
    fn c31_ascii_two_lessons_really_change_combat_and_keep_private_npc_training() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("martial.c31"));
        session.world = World::martial_mentorship(0);
        session.input("martiallog");
        assert!(session.notice.contains("No witnessed"));
        for _ in 0..25 {
            session.input("d");
        }
        session.input("martiallog");
        assert!(!session.notice.contains("Luo Xiu"));
        session.input("drill 35");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::MartialDrilled {
                teacher: ActorId::new(35),
                learned: false
            })
        );
        session.input("d");
        session.input("s");
        session.input("drill 35");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::MartialDrilled {
                teacher: ActorId::new(35),
                learned: true
            })
        );
        session.input("martial");
        assert!(session.notice.contains("StoneFist"));
        session.input("martiallog");
        assert!(session.notice.contains("StoneFist"));
        assert!(!session.notice.contains("Luo Xiu"));
        let prior = session.world.checkpoint();
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), prior);
        session.input("technique 35");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::TechniqueStruck {
                target: ActorId::new(35),
                form: sealed_sim::living::Form::StoneFist
            })
        );
        assert_eq!(
            u16::from_le_bytes(session.world.checkpoint()[12..14].try_into().unwrap()),
            21
        );
        session.input("help");
        assert!(session.notice.contains("C3.1"));
    }

    #[test]
    fn c32_ascii_gathers_real_herbs_and_opens_meridians_in_public_turns() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("meridians.c32"));
        session.world = World::meridian_cultivation(0);
        for _ in 0..8 {
            session.input("d");
        }
        for command in ["w", "w", "g", "g", "g", "s", "s"] {
            session.input(command);
        }
        for _ in 0..5 {
            session.input("a");
        }
        session.input("s");
        assert_eq!(session.world.player().herbs, 3);
        for command in ["drawqi", "circulate", "drawqi", "circulate", "drawqi"] {
            session.input(command);
            assert!(
                !matches!(session.world.latest_outcome(), Some(Outcome::Rejected(_))),
                "{command}: {}",
                session.notice
            );
        }
        let state = session.world.meridians().unwrap().people[&PLAYER];
        assert_eq!((state.qi, state.channels, state.blockage), (1, 2, 0));
        assert_eq!(session.world.player().herbs, 0);
        let saved = session.world.checkpoint();
        session.input("meridianlog");
        assert!(session.notice.contains("Opened"));
        assert!(!session.notice.contains("t1 "));
        session.input("meridians");
        assert!(session.notice.contains("Qi: 1; open channels: 2"));
        assert_eq!(session.world.checkpoint(), saved);
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        assert_eq!(u16::from_le_bytes(saved[12..14].try_into().unwrap()), 22);
        session.input("help");
        assert!(session.notice.contains("C3.2"));
    }

    #[test]
    fn c33_ascii_public_assault_leaves_a_witnessed_wound_and_replays() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("wounds.c33"));
        session.world = World::persistent_injury(0);
        for command in ["w", "d", "d", "attack 1"] {
            session.input(command);
        }
        for (command, times) in [
            ("w", 3),
            ("d", 9),
            ("s", 2),
            ("w", 2),
            ("a", 6),
            ("s", 3),
            ("a", 2),
        ] {
            for _ in 0..times {
                session.input(command);
            }
        }
        session.input("attack 1");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Struck(ActorId::new(1)))
        );
        session.input("woundlog");
        assert!(session.notice.contains("Yao Lian"));
        assert!(session.notice.contains("Legs"));
        session.input("wounds");
        assert!(session.notice.contains("Arm wounds: 0; leg wounds: 0"));
        let prior = session.world.checkpoint();
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), prior);
        assert_eq!(u16::from_le_bytes(prior[12..14].try_into().unwrap()), 23);
        session.input("help");
        assert!(session.notice.contains("C3.3"));
    }

    #[test]
    fn c34_ascii_public_injury_drives_work_paid_care_and_private_observation() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("agency.c34"));
        session.world = World::autonomous_agency(0);
        for command in ["w", "d", "d", "attack 1"] {
            session.input(command);
        }
        for (command, times) in [
            ("w", 3),
            ("d", 9),
            ("s", 2),
            ("w", 2),
            ("a", 6),
            ("s", 3),
            ("a", 2),
        ] {
            for _ in 0..times {
                session.input(command);
            }
        }
        session.input("attack 1");
        assert!(session.world.injuries().unwrap().people[&ActorId::new(1)].injured());

        let observed = session.world.checkpoint();
        session.input("agencylog");
        assert!(!session.notice.is_empty());
        assert_eq!(session.world.checkpoint(), observed);

        let mut unfinished_checkpoint = None;
        for _ in 0..300 {
            session.input("wait");
            let patient = session.world.character_agency().unwrap().people[&ActorId::new(1)];
            if unfinished_checkpoint.is_none()
                && patient.coins > 0
                && session.world.injuries().unwrap().people[&ActorId::new(1)].injured()
            {
                let checkpoint = session.world.checkpoint();
                assert_eq!(World::restore(&checkpoint).unwrap(), session.world);
                unfinished_checkpoint = Some(checkpoint);
            }
            if !session.world.injuries().unwrap().people[&ActorId::new(1)].injured() {
                break;
            }
        }
        assert!(unfinished_checkpoint.is_some());
        assert!(!session.world.injuries().unwrap().people[&ActorId::new(1)].injured());
        let history = &session.world.character_agency().unwrap().history;
        assert!(history.iter().any(|event| {
            event.actor == ActorId::new(1) && event.kind == AgencyEventKind::WorkGathered
        }));
        assert!(history.iter().any(|event| {
            event.actor == ActorId::new(1)
                && matches!(
                    event.kind,
                    AgencyEventKind::CarePaid | AgencyEventKind::CareBartered
                )
        }));

        let inspected = session.world.checkpoint();
        session.input("agency 1");
        assert!(!session.notice.contains("coin(s)"));
        assert!(!session.notice.contains("ambition"));
        assert_eq!(session.world.checkpoint(), inspected);
        session.input("lang id");
        session.input("agencylog");
        assert!(!session.notice.contains("coin(s)"));
        assert!(!session.notice.contains("Some("));
        session.input("lang en");

        session.input("agency 34");
        assert!(session.notice.contains("not visible"));
        let saved = session.world.checkpoint();
        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), saved);
        assert_eq!(u16::from_le_bytes(saved[12..14].try_into().unwrap()), 24);
        session.input("help");
        assert!(session.notice.contains("C3.4"));

        let c33 = World::persistent_injury(0).checkpoint();
        assert_eq!(u16::from_le_bytes(c33[12..14].try_into().unwrap()), 23);
        assert_eq!(
            u16::from_le_bytes(
                session.new_seeded_world(7).checkpoint()[12..14]
                    .try_into()
                    .unwrap()
            ),
            24
        );
    }

    fn ascii_walk_to(session: &mut Session, destination: Position, reach: u32) {
        use std::collections::{BTreeMap, VecDeque};

        for _ in 0..240 {
            let start = session.world.player().position;
            if start.x.abs_diff(destination.x) + start.y.abs_diff(destination.y) <= reach {
                return;
            }
            let scenario = session.world.scenario();
            let mut queue = VecDeque::from([start]);
            let mut parent: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
            let mut goal = None;
            while let Some(pos) = queue.pop_front() {
                if pos.x.abs_diff(destination.x) + pos.y.abs_diff(destination.y) <= reach {
                    goal = Some(pos);
                    break;
                }
                let neighbors = [
                    pos.y.checked_sub(1).map(|y| Position { x: pos.x, y }),
                    pos.x.checked_add(1).map(|x| Position { x, y: pos.y }),
                    pos.y.checked_add(1).map(|y| Position { x: pos.x, y }),
                    pos.x.checked_sub(1).map(|x| Position { x, y: pos.y }),
                ];
                for next in neighbors.into_iter().flatten() {
                    if next.x >= scenario.width || next.y >= scenario.height {
                        continue;
                    }
                    let key = (next.x, next.y);
                    if parent.contains_key(&key) || !session.world.walkable(next) {
                        continue;
                    }
                    if session
                        .world
                        .npcs()
                        .any(|npc| npc.alive() && npc.position == next)
                    {
                        continue;
                    }
                    parent.insert(key, (pos.x, pos.y));
                    queue.push_back(next);
                }
            }
            let command = if let Some(mut cur) = goal {
                while let Some(&prev) = parent.get(&(cur.x, cur.y)) {
                    if prev == (start.x, start.y) {
                        break;
                    }
                    cur = Position {
                        x: prev.0,
                        y: prev.1,
                    };
                }
                if cur.y < start.y {
                    "w"
                } else if cur.x > start.x {
                    "d"
                } else if cur.y > start.y {
                    "s"
                } else if cur.x < start.x {
                    "a"
                } else {
                    "wait"
                }
            } else {
                "wait"
            };
            session.input(command);
        }
        panic!("could not walk to {destination:?}");
    }

    fn ascii_wait_for_open_hall(session: &mut Session) {
        let hall = Region::sites().hall;
        for _ in 0..1_200 {
            if !session.world.is_night()
                && !session
                    .world
                    .rivalry()
                    .unwrap()
                    .contested(session.world.tick())
                && session
                    .world
                    .school()
                    .and_then(|school| school.leader)
                    .and_then(|leader| session.world.npc(leader))
                    .is_some_and(|npc| npc.alive() && npc.position == hall)
            {
                return;
            }
            session.input("wait");
        }
        panic!("Hall mentor did not return to duty");
    }

    fn ascii_prepare_hall_offering(session: &mut Session) {
        let hall = Region::sites().hall;
        let garden = Region::sites().garden;
        let patch = session.world.scenario().patch;
        for _ in 0..3 {
            ascii_wait_for_open_hall(session);
            if session.world.region().unwrap().hall_offerings > 0 {
                return;
            }
            ascii_walk_to(session, garden, 1);
            while session.world.player().herbs < 2
                && session.world.region().unwrap().garden_herbs > 0
            {
                session.input("g");
            }
            if session.world.player().herbs == 0 {
                ascii_walk_to(session, patch, 1);
                session.input("g");
            }
            ascii_walk_to(session, hall, 1);
            ascii_wait_for_open_hall(session);
            while session.world.player().herbs > 0
                && session.world.region().unwrap().hall_offerings < 2
            {
                session.input("sponsor");
            }
            ascii_wait_for_open_hall(session);
            if session.world.region().unwrap().hall_offerings > 0 {
                return;
            }
        }
        panic!("could not stock Hall offerings via public commands");
    }

    #[test]
    fn c35_ascii_sect_hierarchy_advancement_and_allocation() {
        let directory = tempfile::tempdir().unwrap();
        let mut session = Session::new(directory.path().join("sect.c35"));
        session.world = World::sect_hierarchy_campaign(0);

        let before = session.world.checkpoint();
        session.input("sect");
        assert!(session.notice.contains("Not enrolled"));
        assert!(!session.notice.contains("Ren Zhi"));
        session.input("sectlog");
        assert!(session.notice.contains("No witnessed"));
        assert_eq!(session.world.checkpoint(), before);

        let patch = session.world.scenario().patch;
        let shrine = session.world.scenario().shrine;
        let hall = Region::sites().hall;

        ascii_walk_to(&mut session, patch, 1);
        session.input("g");
        session.input("g");
        session.input("g");
        ascii_walk_to(&mut session, shrine, 1);
        session.input("m");

        ascii_walk_to(&mut session, hall, 1);
        ascii_wait_for_open_hall(&mut session);
        session.input("join");
        assert_eq!(session.world.hall_rank(), Some(HallRank::Apprentice));

        session.input("drawqi");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::RankRequired))
        );
        session.input("allocate");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::RankRequired))
        );
        assert!(session.frame().contains("disciple"));

        // First duty -> Disciple
        ascii_prepare_hall_offering(&mut session);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyAccepted));
        ascii_walk_to(&mut session, shrine, 1);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyDelivered));
        ascii_walk_to(&mut session, hall, 1);
        ascii_wait_for_open_hall(&mut session);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyReported));
        assert_eq!(session.world.hall_rank(), Some(HallRank::Disciple));

        // Disciple unlocks Hall Qi access and free Reed Hall drills, but not Senior allocation.
        if session.world.player().herbs == 0 {
            ascii_walk_to(&mut session, patch, 1);
            session.input("g");
            ascii_walk_to(&mut session, hall, 1);
        }
        session.input("drawqi");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::QiDrawn(1)));
        ascii_wait_for_open_hall(&mut session);
        let coins_before = session.world.region().unwrap().coins;
        session.input("drill 33");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::MartialDrilled {
                teacher: ActorId::new(33),
                learned: false
            })
        );
        assert_eq!(session.world.region().unwrap().coins, coins_before);
        session.input("allocate");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::Rejected(Rejection::RankRequired))
        );
        assert!(session.frame().contains("senior disciple"));

        // Second duty -> SeniorDisciple -> claim physical herb allocation
        ascii_prepare_hall_offering(&mut session);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyAccepted));
        ascii_walk_to(&mut session, shrine, 1);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyDelivered));
        ascii_walk_to(&mut session, hall, 1);
        ascii_wait_for_open_hall(&mut session);
        session.input("duty");
        assert_eq!(session.world.latest_outcome(), Some(Outcome::DutyReported));
        assert_eq!(session.world.hall_rank(), Some(HallRank::SeniorDisciple));

        ascii_prepare_hall_offering(&mut session);
        let herbs_before = session.world.player().herbs;
        session.input("allocate");
        assert_eq!(
            session.world.latest_outcome(),
            Some(Outcome::HallAllocationClaimed)
        );
        assert_eq!(session.world.player().herbs, herbs_before + 1);

        let inspected = session.world.checkpoint();
        session.input("sect");
        assert!(session.notice.contains("Reed Hall senior disciple"));
        assert!(session.notice.contains("2 merit"));
        assert!(session.notice.contains("claimed"));
        session.input("sectlog");
        assert!(session.notice.contains("promoted to"));
        assert!(
            session
                .notice
                .contains("claimed a physical Hall herb allocation")
        );
        session.input("lang id");
        session.input("sect");
        assert!(session.notice.contains("Murid senior Balai Alang-alang"));
        session.input("sectlog");
        assert!(session.notice.contains("naik pangkat menjadi"));
        session.input("lang en");
        assert_eq!(session.world.checkpoint(), inspected);

        session.input("save");
        session.input("wait");
        session.input("load");
        assert_eq!(session.world.checkpoint(), inspected);
        assert_eq!(
            u16::from_le_bytes(inspected[12..14].try_into().unwrap()),
            25
        );
        session.input("help");
        assert!(session.notice.contains("C3.5"));

        assert_eq!(
            u16::from_le_bytes(
                session.new_seeded_world(7).checkpoint()[12..14]
                    .try_into()
                    .unwrap()
            ),
            25
        );

        let mut legacy = Session::new(directory.path().join("legacy.c34"));
        legacy.world = World::autonomous_agency(0);
        let legacy_before = legacy.world.checkpoint();
        legacy.input("allocate");
        assert!(legacy.notice.contains("C3.5"));
        assert_eq!(legacy.world.checkpoint(), legacy_before);
        assert_eq!(
            u16::from_le_bytes(legacy_before[12..14].try_into().unwrap()),
            24
        );
    }
}
