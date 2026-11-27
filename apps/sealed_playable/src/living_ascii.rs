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

use crate::{
    clock,
    locale::Locale,
    locale_living::{living_format, living_plural, living_text},
    persistence,
};

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
        if self.world.checkpoint_revision() == 28 {
            World::expanded_riverlands(seed)
        } else if self.world.checkpoint_revision() == 27 {
            World::generated_riverlands(seed)
        } else if self.world.checkpoint_revision() == 26 {
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
            self.notice =
                living_text(self.locale, "living.ascii.input.use_one_command_at_a_time").into();
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
                self.notice = living_text(
                    self.locale,
                    "living.ascii.input.hall_allocations_require_a_new_c3_5",
                )
                .into();
                return true;
            }
            "sect" if self.world.sect_hierarchy().is_some() => {
                let hierarchy = self.world.sect_hierarchy().unwrap();
                let mut parts = Vec::new();
                if let Some(player) = hierarchy.player {
                    let duty = match player.duty {
                        SectDuty::Unassigned => living_text(self.locale, "living.ascii.input.none"),
                        SectDuty::Carrying => {
                            living_text(self.locale, "living.ascii.input.carrying_offering")
                        }
                        SectDuty::Delivered => {
                            living_text(self.locale, "living.ascii.input.awaiting_report")
                        }
                    };
                    let allocation =
                        if !matches!(player.rank, HallRank::SeniorDisciple | HallRank::Elder) {
                            living_text(self.locale, "living.ascii.input.requires_senior_disciple")
                        } else if player.allocation_claimed {
                            living_text(self.locale, "living.ascii.input.claimed")
                        } else {
                            living_text(self.locale, "living.ascii.input.available")
                        };
                    parts.push(living_format(
                        self.locale,
                        "living.ascii.input.template.reed_hall_rank_player_merit_merit",
                        &[
                            ("rank", hall_rank_label(self.locale, player.rank)),
                            ("player_merit", &format!("{}", player.merit)),
                            ("duty_2", duty),
                            ("allocation_2", allocation),
                        ],
                    ));
                } else {
                    parts.push(
                        living_text(self.locale, "living.ascii.input.not_enrolled_in_reed_hall")
                            .into(),
                    );
                }
                for (&id, &member) in &hierarchy.members {
                    if let Some(npc) = self.world.npc(id)
                        && npc.alive()
                        && self.world.player_sees(npc.position)
                    {
                        parts.push(living_format(
                            self.locale,
                            "living.ascii.input.template.npc_definition_name_rank",
                            &[
                                ("npc_definition_name", npc.definition.name),
                                ("rank", hall_rank_label(self.locale, member.rank)),
                            ],
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
                            HierarchyEventKind::DutyAccepted => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_accepted",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                ],
                            ),
                            HierarchyEventKind::DutyDelivered => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_delivered",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                ],
                            ),
                            HierarchyEventKind::MeritEarned => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_earned",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    ("event_merit", &format!("{}", event.merit)),
                                ],
                            ),
                            HierarchyEventKind::Promoted => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_promoted",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    ("rank", rank),
                                ],
                            ),
                            HierarchyEventKind::AllocationClaimed => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_claimed",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                ],
                            ),
                            HierarchyEventKind::OfficeAssumed => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_assumed",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    ("rank", rank),
                                ],
                            ),
                        }
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_sect_hierarchy_events",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "wounds" if self.world.injuries().is_some() => {
                let state = self.world.injuries().unwrap().people[&PLAYER];
                self.notice = living_format(
                    self.locale,
                    "living.ascii.input.template.state_arms_state_legs",
                    &[
                        ("state_arms", &format!("{}", state.arms)),
                        ("state_legs", &format!("{}", state.legs)),
                    ],
                );
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
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_id_event_at_actor_event",
                            &[
                                ("event_id", &format!("{}", event.id)),
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                ("event_kind", &format!("{:?}", event.kind)),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(self.locale, "living.ascii.woundlog.empty").into()
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
                    self.notice = living_text(
                        self.locale,
                        "living.ascii.input.that_character_is_not_visible",
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
                        living_format(
                            self.locale,
                            "living.ascii.input.template.npc_definition_name_you_have_not",
                            &[("npc_definition_name", npc.definition.name)],
                        )
                    },
                    |event| {
                        living_format(
                            self.locale,
                            "living.ascii.input.template.npc_definition_name_you_observed_event",
                            &[
                                ("npc_definition_name", npc.definition.name),
                                (
                                    "event_goal_map_goal_goal_label",
                                    event
                                        .goal
                                        .map(|goal| goal_label(self.locale, goal))
                                        .unwrap_or("?"),
                                ),
                                (
                                    "event_reason_map_reason_decision_reason",
                                    event
                                        .reason
                                        .map(|reason| decision_reason_label(self.locale, reason))
                                        .unwrap_or("?"),
                                ),
                            ],
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
                            AgencyEventKind::GoalChanged => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_event_2",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    (
                                        "event_goal_map_goal_goal_label",
                                        event
                                            .goal
                                            .map(|goal| goal_label(self.locale, goal))
                                            .unwrap_or("?"),
                                    ),
                                    (
                                        "event_reason_map_reason_decision_reason",
                                        event
                                            .reason
                                            .map(|reason| {
                                                decision_reason_label(self.locale, reason)
                                            })
                                            .unwrap_or("?"),
                                    ),
                                ],
                            ),
                            AgencyEventKind::WorkGathered => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_gathered",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                ],
                            ),
                            AgencyEventKind::WorkSold => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_sold",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    ("event_amount", &format!("{}", event.amount)),
                                ],
                            ),
                            AgencyEventKind::CarePaid => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_paid",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                    ("event_amount", &format!("{}", event.amount)),
                                ],
                            ),
                            AgencyEventKind::CareBartered => living_format(
                                self.locale,
                                "living.ascii.input.template.event_id_event_at_actor_bartered",
                                &[
                                    ("event_id", &format!("{}", event.id)),
                                    ("event_at", &format!("{}", event.at)),
                                    ("actor", actor),
                                ],
                            ),
                        }
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.you_have_witnessed_no_character_decisions",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "meridians" if self.world.meridians().is_some() => {
                let state = self.world.meridians().unwrap().people[&PLAYER];
                self.notice = living_format(
                    self.locale,
                    "living.ascii.input.template.state_qi_state_channels_state_blockage",
                    &[
                        ("state_qi", &format!("{}", state.qi)),
                        ("state_channels", &format!("{}", state.channels)),
                        ("state_blockage", &format!("{}", state.blockage)),
                    ],
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
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_id_event_at_actor_event_3",
                            &[
                                ("event_id", &format!("{}", event.id)),
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                ("event_kind", &format!("{:?}", event.kind)),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(self.locale, "living.ascii.meridianlog.empty").into()
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
                        living_text(self.locale, "living.ascii.input.tell_npc_event").into();
                    return true;
                };
                let Some(event) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.tell_npc_event").into();
                    return true;
                };
                Intent::Testify {
                    to: ActorId::new(actor),
                    event: sealed_sim::living::EventId(event),
                }
            }
            "reportbody" => {
                if !self.world.is_player_reporting() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_7").into();
                    return true;
                }
                let Some(body) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.reportbody_npc_body").into();
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
                    self.notice = living_text(self.locale, "living.ascii.input.claim_lot").into();
                    return true;
                };
                if self.world.contested().is_none() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_9").into();
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
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c3_0").into();
                    return true;
                }
                Intent::AttuneLot(id)
            }
            "cultivation" => {
                if !self.world.is_sect_cultivation() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c3_0").into();
                    return true;
                }
                let state = self.world.contested().unwrap();
                self.notice = living_format(
                    self.locale,
                    "living.ascii.input.template.state_wards_state_script_charges",
                    &[
                        ("state_wards", &format!("{}", state.wards)),
                        ("state_script_charges", &format!("{}", state.script_charges)),
                    ],
                );
                return true;
            }
            "martial" => {
                let Some(martial) = self.world.martial() else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c3_1").into();
                    return true;
                };
                let skill = martial.skills[&PLAYER];
                self.notice = living_format(
                    self.locale,
                    "living.ascii.input.template.skill_form_skill_lessons_skill_strain",
                    &[
                        ("skill_form", &format!("{:?}", skill.form)),
                        ("skill_lessons", &format!("{}", skill.lessons)),
                        ("skill_strain", &format!("{}", skill.strain)),
                        ("skill_parry", &format!("{}", skill.parry)),
                    ],
                );
                return true;
            }
            "martiallog" => {
                let Some(martial) = self.world.martial() else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c3_1").into();
                    return true;
                };
                let lines: Vec<_> = martial
                    .lessons
                    .iter()
                    .filter(|lesson| lesson.witnessed)
                    .map(|lesson| {
                        living_format(
                            self.locale,
                            "living.ascii.input.template.lesson_id_lesson_at_actor_actor",
                            &[
                                ("lesson_id", &format!("{}", lesson.id)),
                                ("lesson_at", &format!("{}", lesson.at)),
                                (
                                    "actor",
                                    actor_name(self.locale, &self.world, lesson.student),
                                ),
                                (
                                    "actor_2",
                                    actor_name(self.locale, &self.world, lesson.teacher),
                                ),
                                ("lesson_form", &format!("{:?}", lesson.form)),
                                (
                                    "conditional",
                                    if lesson.learned {
                                        living_text(self.locale, "living.ascii.martiallog.learned")
                                    } else {
                                        ""
                                    },
                                ),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(self.locale, "living.ascii.martiallog.empty").into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "offerlot" => {
                let Some(id) = second.and_then(|value| value.parse::<u32>().ok()) else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.offerlot_npc_lot").into();
                    return true;
                };
                if self.world.contested().is_none() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_9").into();
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
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_9").into();
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
                                living_text(self.locale, "living.ascii.input.your_pack").into()
                            }
                            _ => return None,
                        };
                        Some(living_format(
                            self.locale,
                            "living.ascii.input.template.lot_id_lot_good_place",
                            &[
                                ("lot_id", &format!("{}", lot.id)),
                                ("lot_good", &format!("{:?}", lot.good)),
                                ("place", &place.to_string()),
                            ],
                        ))
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.no_visible_lots_or_carried_relics",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "lotlog" => {
                let Some(state) = self.world.contested() else {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_9").into();
                    return true;
                };
                let lines: Vec<_> = state
                    .history
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        let verb = match event.kind {
                            LotEventKind::Claimed => {
                                living_text(self.locale, "living.ascii.input.claimed_2")
                            }
                            LotEventKind::Sold => {
                                living_text(self.locale, "living.ascii.input.sold")
                            }
                            LotEventKind::Deposited => {
                                living_text(self.locale, "living.ascii.input.deposited")
                            }
                            LotEventKind::Dropped => {
                                living_text(self.locale, "living.ascii.input.dropped")
                            }
                            LotEventKind::Attuned => {
                                living_text(self.locale, "living.ascii.input.attuned")
                            }
                        };
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_id_event_at_actor_verb",
                            &[
                                ("event_id", &format!("{}", event.id)),
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                ("verb", verb),
                                ("event_lot", &format!("{}", event.lot)),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(self.locale, "living.ascii.input.no_witnessed_lot_transfers").into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "know" => {
                if !self.world.is_player_agency() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.know_is_part_of_the_c2_3")
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
                                        living_text(self.locale, "living.ascii.input.witnessed")
                                    }
                                    Provenance::Reported { sender, .. } if sender == PLAYER => {
                                        living_text(self.locale, "living.ascii.input.examined")
                                    }
                                    Provenance::Reported { hops, .. } => {
                                        if hops <= 1 {
                                            living_text(
                                                self.locale,
                                                "living.ascii.input.provenance.heard",
                                            )
                                        } else {
                                            living_text(
                                                self.locale,
                                                "living.ascii.input.provenance.rumor",
                                            )
                                        }
                                    }
                                };
                                let finding = match event.finding {
                                    AuditFinding::Delivered => {
                                        living_text(self.locale, "living.ascii.input.delivered")
                                    }
                                    AuditFinding::Unfulfilled => {
                                        living_text(self.locale, "living.ascii.input.missed")
                                    }
                                };
                                Some(living_format(
                                    self.locale,
                                    "living.ascii.input.template.id_finding_actor_how",
                                    &[
                                        ("id", &format!("{}", id.0)),
                                        ("finding", finding),
                                        (
                                            "actor",
                                            actor_name(self.locale, &self.world, event.provider),
                                        ),
                                        ("how", how),
                                    ],
                                ))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.you_know_no_institutional_finding",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "bodies" => {
                if !self.world.is_violence_logistics() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_5").into();
                    return true;
                }
                let lines: Vec<_> = self
                    .world
                    .social()
                    .discoveries()
                    .filter(|discovery| discovery.observer == PLAYER)
                    .map(|discovery| {
                        living_format(
                            self.locale,
                            "living.ascii.input.template.discovery_at_actor_cause_unknown_unless",
                            &[
                                ("discovery_at", &format!("{}", discovery.at)),
                                (
                                    "actor",
                                    actor_name(self.locale, &self.world, discovery.body),
                                ),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.you_have_discovered_no_bodies",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "salvagelog" => {
                if !self.world.is_physical_recovery() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_8").into();
                    return true;
                }
                let lines: Vec<_> = self
                    .world
                    .social()
                    .recoveries()
                    .iter()
                    .filter(|event| event.witnessed)
                    .map(|event| {
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_id_event_at_actor_conditional",
                            &[
                                ("event_id", &format!("{}", event.id)),
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                (
                                    "conditional",
                                    if event.delivered {
                                        living_text(
                                            self.locale,
                                            "living.ascii.input.recovery.delivered",
                                        )
                                    } else {
                                        living_text(
                                            self.locale,
                                            "living.ascii.input.recovery.recovered",
                                        )
                                    },
                                ),
                                ("event_herbs", &format!("{}", event.herbs)),
                                ("actor_2", actor_name(self.locale, &self.world, event.body)),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_cargo_recovery",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "crimelog" => {
                if !self.world.is_violence_logistics() {
                    self.notice =
                        living_text(self.locale, "living.ascii.input.only_in_c2_5").into();
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
                            living_text(self.locale, "living.ascii.input.you_did_this")
                        } else if incident.target == PLAYER {
                            living_text(self.locale, "living.ascii.input.you_were_struck")
                        } else {
                            match self.world.social().knowledge(PLAYER, incident.id)?.source {
                                KnowledgeSource::Observed => {
                                    living_text(self.locale, "living.ascii.input.witnessed")
                                }
                                KnowledgeSource::Reported { hops: 1, .. } => {
                                    living_text(self.locale, "living.ascii.input.heard_firsthand")
                                }
                                KnowledgeSource::Reported { .. } => {
                                    living_text(self.locale, "living.ascii.input.allegation")
                                }
                            }
                        };
                        let action = if incident.damage == 0 {
                            living_text(self.locale, "living.ascii.input.took_cargo_from")
                        } else {
                            living_text(self.locale, "living.ascii.input.hit")
                        };
                        Some(living_format(
                            self.locale,
                            "living.ascii.input.template.incident_tick_actor_action_actor_2",
                            &[
                                ("incident_tick", &format!("{}", incident.tick)),
                                (
                                    "actor",
                                    actor_name(self.locale, &self.world, incident.source),
                                ),
                                ("action", action),
                                (
                                    "actor_2",
                                    actor_name(self.locale, &self.world, incident.target),
                                ),
                                ("relation", relation),
                            ],
                        ))
                    })
                    .take(12)
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.you_know_no_assaults_or_thefts",
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
                        living_format(
                            self.locale,
                            "living.ascii.input.template.world_seed_seed",
                            &[("seed", &format!("{}", seed))],
                        )
                    }
                    _ => living_text(
                        self.locale,
                        "living.ascii.input.use_seed_number_before_the_first_turn",
                    )
                    .into(),
                };
                return true;
            }
            "survey" if self.world.generated_region().is_some() => {
                self.notice = self.survey();
                return true;
            }
            "help" => {
                self.notice = if self.world.checkpoint_revision() >= 26 {
                    living_text(self.locale, "living.help.c351")
                } else if self.world.sect_hierarchy().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c3_5_reed_hall_tracks_persisted_rank",
                    )
                } else if self.world.character_agency().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c3_4_injured_npcs_without_treatment_money",
                    )
                } else if self.world.injuries().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c3_3_real_blows_can_leave_lasting",
                    )
                } else if self.world.meridians().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c3_2_gather_real_herbs_at_qinghe",
                    )
                } else if self.world.martial().is_some() {
                    living_text(self.locale, "living.ascii.input.c3_1_seek_ren_zhi_33_for")
                } else if self.world.is_sect_cultivation() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c3_0_keep_a_relic_to_cultivate",
                    )
                } else if self.world.contested().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_9_rival_sects_compete_for_finite",
                    )
                } else if self.world.is_physical_recovery() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_8_watchmen_who_see_a_body",
                    )
                } else if self.world.is_player_reporting() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_7_find_a_body_then_reportbody",
                    )
                } else if self.world.is_violence_logistics() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_5_attack_id_a_living_courier",
                    )
                } else if self.world.is_consequences() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_4_a_dead_carrier_s_herbs",
                    )
                } else if self.world.is_player_agency() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_3_you_can_witness_examine_a",
                    )
                } else if self.world.information().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_2_audit_findings_travel_only_by",
                    )
                } else if self.world.contract_memory().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_1_institutions_inspect_their_own_delivery",
                    )
                } else if self.world.contracts().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c2_0_qinghe_and_reed_hall_may",
                    )
                } else if self.world.faction_agency_state().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c1_9_stonebank_s_envoy_periodically_visits",
                    )
                } else if self.world.relief().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c1_8_an_observed_qinghe_medicine_shortage",
                    )
                } else if self.world.traffic().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.c1_7_move_into_an_adjacent_friendly",
                    )
                } else if self.world.politics().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.reed_hall_and_stonebank_compete_over_willowford",
                    )
                } else if self.world.region().is_some() {
                    living_text(
                        self.locale,
                        "living.ascii.input.follow_the_eastern_road_to_willowford_gather",
                    )
                } else {
                    living_text(
                        self.locale,
                        "living.ascii.input.gather_herbs_at_the_patch_and_deliver",
                    )
                }
                .into();
                if self.world.generated_region().is_some() {
                    self.notice.push(' ');
                    self.notice
                        .push_str(living_text(self.locale, "living.worldgen.help"));
                }
                if self.world.frontier().is_some() {
                    self.notice.push(' ');
                    self.notice
                        .push_str(living_text(self.locale, "living.expansion.help"));
                }
                return true;
            }
            "lang" => {
                let next = match argument {
                    Some("en") => Locale::English,
                    Some("id") => Locale::Indonesian,
                    _ => {
                        self.notice = "lang en | lang id".into();
                        return true;
                    }
                };
                if next != self.locale {
                    self.locale = next;
                    // A cached notice was authored in the previous language.
                    // Discard presentation-only text; never change world state.
                    self.notice.clear();
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
                            self.notice =
                                living_text(self.locale, "living.ascii.input.only_in_c3_0").into();
                            return true;
                        }
                        Intent::ChannelStrike(id)
                    }
                    "drill" => {
                        if self.world.martial().is_none() {
                            self.notice =
                                living_text(self.locale, "living.ascii.input.only_in_c3_1").into();
                            return true;
                        }
                        Intent::Drill(id)
                    }
                    "technique" => {
                        if self.world.martial().is_none() {
                            self.notice =
                                living_text(self.locale, "living.ascii.input.only_in_c3_1").into();
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
                    Ok(()) => living_text(self.locale, "living.save.ok").into(),
                    Err(error) => {
                        living_format(self.locale, "living.save.error", &[("error", &error)])
                    }
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
                        living_text(self.locale, "living.load.ok").into()
                    }
                    Err(error) => {
                        living_format(self.locale, "living.load.error", &[("error", &error)])
                    }
                };
                return true;
            }
            "r" | "reset" => {
                let seed = self.world.region().map_or(0, |region| region.seed);
                self.world = self.new_seeded_world(seed);
                self.notice = living_text(self.locale, "living.reset.ok").into();
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_recorded_travel_encounters",
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_passage_exchanges",
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
                    living_text(self.locale, "living.ascii.input.no_witnessed_relief_events").into()
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_faction_aid_events",
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_institutional_contracts",
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_contract_audits",
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
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_institutional_information",
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
                                living_text(self.locale, "living.ascii.input.bought_an_herb")
                            }
                            SectEventKind::Petitioned => living_text(
                                self.locale,
                                "living.ascii.input.pressed_a_claim_at_reed_hall",
                            ),
                            SectEventKind::Accord => {
                                living_text(self.locale, "living.ascii.input.negotiated_a_truce")
                            }
                            SectEventKind::Patronage => {
                                living_text(self.locale, "living.ascii.input.supported_reed_hall")
                            }
                        };
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_at_actor_verb",
                            &[
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                ("verb", verb),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.you_witnessed_no_sect_events",
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
                            PoliticalEventKind::HallPurchase => living_text(
                                self.locale,
                                "living.ascii.input.bought_herbs_for_reed_hall",
                            ),
                            PoliticalEventKind::HallSupply => living_text(
                                self.locale,
                                "living.ascii.input.delivered_hall_supplies",
                            ),
                            PoliticalEventKind::LodgeOath => {
                                living_text(self.locale, "living.ascii.input.pledged_to_stonebank")
                            }
                            PoliticalEventKind::LodgeAid => {
                                living_text(self.locale, "living.ascii.input.supplied_stonebank")
                            }
                            PoliticalEventKind::LodgeRenunciation => {
                                living_text(self.locale, "living.ascii.input.renounced_stonebank")
                            }
                            PoliticalEventKind::HallPatronage => {
                                living_text(self.locale, "living.ascii.input.supported_reed_hall")
                            }
                        };
                        living_format(
                            self.locale,
                            "living.ascii.input.template.event_at_actor_action",
                            &[
                                ("event_at", &format!("{}", event.at)),
                                ("actor", actor_name(self.locale, &self.world, event.actor)),
                                ("action", action),
                            ],
                        )
                    })
                    .collect();
                self.notice = if lines.is_empty() {
                    living_text(
                        self.locale,
                        "living.ascii.input.no_witnessed_political_events",
                    )
                    .into()
                } else {
                    lines.join("\n")
                };
                return true;
            }
            "look" => return true,
            _ => {
                self.notice = living_format(
                    self.locale,
                    "living.ascii.input.template.unknown_command_command",
                    &[("command", command)],
                );
                return true;
            }
        };

        if let Err(error) = self.world.step(self.world.tick(), intent) {
            self.notice = match error {
                StepError::UnavailableInOriginalRules => living_text(
                    self.locale,
                    "living.ascii.input.breath_practice_requires_a_new_campaign",
                )
                .into(),
                StepError::RegionalCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.this_action_requires_a_qinghe_willowford_campaign",
                )
                .into(),
                StepError::CompanyCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.travelling_partnerships_require_a_new_campaign",
                )
                .into(),
                StepError::RivalryCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.sect_diplomacy_requires_a_c1_5_campaign",
                )
                .into(),
                StepError::PoliticsCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.allegiance_requires_a_c1_6_campaign",
                )
                .into(),
                StepError::SchoolCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.this_action_requires_a_new_reed_hall",
                )
                .into(),
                StepError::HorizonReached => living_text(
                    self.locale,
                    "living.ascii.input.this_campaign_reached_its_turn_limit",
                )
                .into(),
                StepError::WrongBoundary { .. } => {
                    living_text(self.locale, "living.ascii.input.the_turn_is_out_of_date").into()
                }
                StepError::InvalidBodyReport => living_text(
                    self.locale,
                    "living.ascii.input.invalid_body_report_or_unavailable_campaign",
                )
                .into(),
                StepError::UnrecordableIntent => {
                    living_text(self.locale, "living.intent.unrecordable").into()
                }
                StepError::ContestedCampaignRequired => living_text(
                    self.locale,
                    "living.ascii.input.contested_resources_are_unavailable_in_this_campaign",
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
        living_text(
            self.locale,
            "living.ascii.target_hint.choose_a_visible_npc_by_id",
        )
    }

    fn inspect(&self, argument: Option<&str>) -> String {
        let id = match self.target(argument) {
            Ok(id) => id,
            Err(message) => return message.into(),
        };
        let npc = self.world.npc(id).expect("visible target");
        let tr = |key| living_text(self.locale, key);
        let occupation = match npc.definition.role {
            Role::Medic => tr("living.ascii.inspect.healer"),
            Role::Watch => tr("living.ascii.inspect.watch"),
            Role::Supplier => tr("living.ascii.inspect.herbalist"),
            Role::Traveler => tr("living.ascii.inspect.traveler"),
            Role::Grower => tr("living.ascii.inspect.grower"),
            Role::Courier => tr("living.ascii.inspect.courier"),
            Role::Mentor => tr("living.ascii.inspect.mentor"),
            Role::HallDeputy => tr("living.ascii.inspect.deputy"),
            Role::RivalEnvoy => tr("living.ascii.inspect.stonebank_envoy"),
            Role::HallSteward => tr("living.ascii.inspect.hall_steward"),
            Role::FrontierCarrier => tr("living.expansion.role.carrier"),
            Role::FrontierGrower => tr("living.expansion.role.grower"),
        };
        let condition = if npc.hp < npc.definition.hp {
            tr("living.ascii.inspect.wounded")
        } else {
            tr("living.ascii.inspect.no_visible_wounds")
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
            return living_format(
                self.locale,
                "living.ascii.inspect.template.npc_definition_name_occupation_condition_packed",
                &[
                    ("npc_definition_name", npc.definition.name),
                    ("occupation", occupation),
                    ("condition", condition),
                    ("npc_packed_herbs", &format!("{}", npc.packed_herbs)),
                ],
            );
        }
        let invitation = match self.world.invitation(id) {
            Some(Invitation::OpenRoad) => tr("living.ascii.inspect.talk_then_invite_to_travel"),
            Some(Invitation::SchoolMember) => {
                tr("living.ascii.inspect.talk_first_hall_members_can_invite_the")
            }
            Some(Invitation::LodgeMember) => {
                tr("living.ascii.inspect.talk_first_stonebank_members_may_invite_this")
            }
            None => "",
        };
        living_format(
            self.locale,
            "living.ascii.inspect.template.npc_definition_name_occupation_condition_invitation",
            &[
                ("npc_definition_name", npc.definition.name),
                ("occupation", occupation),
                ("condition", condition),
                ("invitation", invitation),
            ],
        )
    }

    /// Both presentation modes project the same observation rules.
    fn survey(&self) -> String {
        use sealed_sim::living::worldgen::Terrain;
        let player = self.world.player();
        let Some(map) = self.world.generated_region().filter(|_| player.alive()) else {
            return living_text(self.locale, "living.worldgen.no_survey").into();
        };
        let Some(tile) = self.world.terrain_tile(player.position) else {
            return living_text(self.locale, "living.worldgen.no_survey").into();
        };
        let key = match tile.terrain {
            Terrain::Ground => "living.worldgen.terrain.ground",
            Terrain::Wall => "living.worldgen.terrain.wall",
            Terrain::Meadow => "living.worldgen.terrain.meadow",
            Terrain::Woodland => "living.worldgen.terrain.woodland",
            Terrain::Rock => "living.worldgen.terrain.rock",
            Terrain::River => "living.worldgen.terrain.river",
            Terrain::Road => "living.worldgen.terrain.road",
            Terrain::Bridge => "living.worldgen.terrain.bridge",
        };
        let mut result = living_format(
            self.locale,
            "living.worldgen.survey",
            &[
                ("terrain", living_text(self.locale, key)),
                ("elevation", &tile.elevation.to_string()),
                ("moisture", &tile.moisture.to_string()),
                ("qi", &tile.qi.to_string()),
            ],
        );
        let near = |site: Position| {
            player.position.x.abs_diff(site.x) + player.position.y.abs_diff(site.y) <= 1
                && self.world.player_sees(site)
        };
        let cadence = if near(sealed_sim::living::expansion::GROVE)
            && self.world.expanded_region().is_some()
        {
            self.world
                .expanded_region()
                .map(|map| map.grove_regrow_every())
        } else if near(self.world.scenario().patch) {
            Some(map.west_regrow_every())
        } else if near(Region::sites().garden) {
            Some(map.garden_regrow_every())
        } else {
            None
        };
        if let Some(cadence) = cadence {
            result.push(' ');
            result.push_str(&living_format(
                self.locale,
                "living.worldgen.regrowth",
                &[("turns", &cadence.to_string())],
            ));
        }
        result
    }

    fn ground_symbol(&self, cell: Position) -> char {
        if let Some(tile) = self.world.terrain_tile(cell) {
            tile.terrain.glyph()
        } else if self.world.walkable(cell) {
            '.'
        } else {
            '#'
        }
    }

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
            return if wall_edge {
                self.ground_symbol(cell)
            } else {
                '?'
            };
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
        if world.frontier().is_some() {
            use sealed_sim::living::expansion::{DEPOT, GROVE};
            if cell == DEPOT {
                return '$';
            }
            if cell == GROVE {
                return '&';
            }
        }
        self.ground_symbol(cell)
    }

    pub(crate) fn frontier_hint(&self) -> Option<String> {
        use sealed_sim::living::expansion::{DEPOT, GROVE};
        let f = self.world.frontier()?;
        let visible = |p| self.debug_world || self.world.player_sees(p);
        if visible(DEPOT) {
            if self.world.is_night() {
                Some(living_format(
                    self.locale,
                    "living.expansion.depot_closed",
                    &[("stock", &f.depot_herbs.to_string())],
                ))
            } else {
                let q = f.quote();
                Some(living_format(
                    self.locale,
                    "living.expansion.depot",
                    &[
                        ("stock", &f.depot_herbs.to_string()),
                        ("buy", &q.buy.to_string()),
                        ("sell", &q.sell.to_string()),
                    ],
                ))
            }
        } else if visible(GROVE) {
            Some(living_format(
                self.locale,
                "living.expansion.grove",
                &[("stock", &f.grove_herbs.to_string())],
            ))
        } else {
            None
        }
    }

    fn frame(&self) -> String {
        let world = &self.world;
        let scenario = world.scenario();
        let player = world.player();
        let tr = |key| living_text(self.locale, key);
        let visible = |cell| self.debug_world || world.player_sees(cell);
        let (hour, minute) = clock::clock_hhmm(clock::minute_of_day(world.calendar_tick()));
        let mut out = living_format(
            self.locale,
            "living.ascii.frame.template.scenario_name_day_clock_day_number",
            &[
                ("scenario_name", scenario.name),
                (
                    "clock_day_number_world_calendar_tick",
                    &format!("{}", clock::day_number(world.calendar_tick())),
                ),
                ("hour", &format!("{:02}", hour)),
                ("minute", &format!("{:02}", minute)),
                ("world_tick", &format!("{}", world.tick())),
                ("player_hp", &format!("{}", player.hp)),
                ("player_herbs", &format!("{}", player.herbs)),
                (
                    "conditional",
                    if player.alive() {
                        ""
                    } else {
                        tr("living.ascii.frame.dead_n_advances_time")
                    },
                ),
            ],
        );
        if self.debug_world {
            let _ = writeln!(out, "{}", tr("living.ascii.frame.developer_view"));
        }
        if world.original_rules() {
            let _ = writeln!(out, "{}", tr("living.ascii.frame.original_c1_0_campaign"));
        } else {
            let _ = writeln!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.breath_world_breath_progress_breath_mastery",
                    &[
                        (
                            "world_breath_progress",
                            &format!("{}", world.breath_progress())
                        ),
                        ("breath_mastery", &format!("{}", BREATH_MASTERY))
                    ]
                )
            );
        }
        let sites = world.region().map(|_| Region::sites());
        if let Some(region) = world.region() {
            let _ = writeln!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.coins_region_coins_remedies_region_remedies",
                    &[
                        ("region_coins", &format!("{}", region.coins)),
                        ("region_remedies", &format!("{}", region.remedies)),
                        ("region_seed", &format!("{}", region.seed))
                    ]
                )
            );
            if let Some(background) = region.background {
                let _ = writeln!(out, "{}", background_label(self.locale, background));
            } else {
                let _ = writeln!(
                    out,
                    "{}",
                    tr("living.ascii.frame.before_moving_choose_background_wanderer_3_coins")
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
                let _ = writeln!(out, "{}", tr("living.ascii.frame.stonebank_lodge_member"));
            }
        }
        if let Some(school) = world.school() {
            if school.form_learned {
                let _ = writeln!(
                    out,
                    "{}",
                    living_format(
                        self.locale,
                        "living.ascii.frame.template.school_qi_max_qi",
                        &[
                            ("school_qi", &format!("{}", school.qi)),
                            ("max_qi", &format!("{}", MAX_QI))
                        ]
                    )
                );
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
        let height = if self.debug_world {
            scenario.height
        } else {
            scenario.height.min(9)
        };
        let top = player
            .position
            .y
            .saturating_sub(height / 2)
            .min(scenario.height - height);
        for y in top..top + height {
            for x in left..left + width {
                let glyph = self.tile_symbol(Position { x, y });
                let _ = write!(out, "{glyph} ");
            }
            out.push('\n');
        }
        let _ = writeln!(
            out,
            "{}",
            tr("living.ascii.frame.you_herb_patch_shrine_clinic_wall_x")
        );
        if let Some(map) = world.generated_region() {
            let _ = writeln!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.worldgen.legend",
                    &[("seed", &map.seed().to_string())]
                )
            );
        }
        if world.frontier().is_some() {
            let _ = writeln!(
                out,
                "{}",
                living_text(self.locale, "living.expansion.legend")
            );
            if let Some(hint) = self.frontier_hint() {
                let _ = writeln!(out, "{hint}");
            }
        }
        for npc in world.npcs().filter(|npc| visible(npc.position)) {
            let status = if !npc.alive() {
                tr("living.ascii.frame.fallen")
            } else if self.debug_world {
                goal_label(self.locale, npc.activity.goal)
            } else if world
                .company()
                .and_then(|company| company.journey)
                .is_some_and(|j| j.actor == npc.id())
            {
                match npc.activity.goal {
                    Goal::Rest => tr("living.ascii.frame.companion_resting"),
                    Goal::SeekCare => tr("living.ascii.frame.companion_seeking_care"),
                    _ => tr("living.ascii.frame.travelling_companion"),
                }
            } else if npc.herbs > 0 {
                tr("living.ascii.frame.carrying_herbs")
            } else if npc.definition.role == Role::Medic && npc.position == scenario.clinic {
                tr("living.ascii.frame.at_the_clinic")
            } else {
                ""
            };
            let _ = write!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.npc_definition_glyph_npc_id_get",
                    &[
                        ("npc_definition_glyph", &format!("{}", npc.definition.glyph)),
                        ("npc_id_get", &format!("{:>2}", npc.id().get())),
                        ("npc_definition_name", npc.definition.name),
                        ("npc_position_x", &format!("{}", npc.position.x)),
                        ("npc_position_y", &format!("{}", npc.position.y))
                    ]
                )
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
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.shrine_stock_supply_shrine_scenario_shrine",
                    &[
                        ("supply_shrine", &format!("{}", supply.shrine)),
                        (
                            "scenario_shrine_target",
                            &format!("{}", scenario.shrine_target)
                        )
                    ]
                )
            );
        }
        if visible(scenario.patch) {
            let _ = writeln!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.herbs_in_patch_supply_source",
                    &[("supply_source", &format!("{}", supply.source))]
                )
            );
        }
        if let Some((region, sites)) = world.region().zip(sites) {
            if visible(sites.market) {
                let quote = world.market_quote().expect("region present");
                if world.is_night() {
                    let _ = writeln!(
                        out,
                        "{}",
                        tr("living.ascii.frame.willowford_market_closed_until_dawn")
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "{}",
                        living_format(
                            self.locale,
                            "living.ascii.frame.template.market_herbs_region_market_herbs_buy",
                            &[
                                ("region_market_herbs", &format!("{}", region.market_herbs)),
                                ("quote_buy", &format!("{}", quote.buy)),
                                ("quote_sell", &format!("{}", quote.sell))
                            ]
                        )
                    );
                }
            }
            if visible(sites.garden) {
                let _ = writeln!(
                    out,
                    "{}",
                    living_format(
                        self.locale,
                        "living.ascii.frame.template.willowford_garden_herbs_region_garden_herbs",
                        &[("region_garden_herbs", &format!("{}", region.garden_herbs))]
                    )
                );
            }
            if visible(sites.hall) {
                let _ = writeln!(out, "{}", self.hall_hint());
                if let Some(rivalry) = world.rivalry() {
                    let state = if rivalry.at_peace(world.tick()) {
                        tr("living.ascii.frame.truce")
                    } else if rivalry.contested(world.tick()) {
                        tr("living.ascii.frame.contested")
                    } else {
                        tr("living.ascii.frame.stable")
                    };
                    let _ = writeln!(
                        out,
                        "{}",
                        living_format(
                            self.locale,
                            "living.ascii.frame.template.hall_influence_state",
                            &[("state", state)]
                        )
                    );
                }
            }
            if self.debug_world {
                let _ = writeln!(
                    out,
                    "{}",
                    living_format(
                        self.locale,
                        "living.ascii.frame.template.herbs_shipped_to_qinghe_region_shipped",
                        &[(
                            "region_shipped_to_qinghe",
                            &format!("{}", region.shipped_to_qinghe)
                        )]
                    )
                );
            }
        }
        if self.debug_world {
            let _ = writeln!(
                out,
                "{}",
                living_format(
                    self.locale,
                    "living.ascii.frame.template.npc_deliveries_supply_npc_deliveries_used",
                    &[
                        (
                            "supply_npc_deliveries",
                            &format!("{}", supply.npc_deliveries)
                        ),
                        ("supply_consumed", &format!("{}", supply.consumed))
                    ]
                )
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
                    "{}",
                    living_format(
                        self.locale,
                        "living.ascii.frame.template.incident_tick_actor_took_cargo_from",
                        &[
                            ("incident_tick", &format!("{}", incident.tick)),
                            ("actor", actor_name(self.locale, world, incident.source)),
                            ("actor_2", actor_name(self.locale, world, incident.target))
                        ]
                    )
                );
            } else {
                let _ = writeln!(
                    out,
                    "{}",
                    living_format(
                        self.locale,
                        "living.ascii.frame.template.incident_tick_actor_hit_actor_2",
                        &[
                            ("incident_tick", &format!("{}", incident.tick)),
                            ("actor", actor_name(self.locale, world, incident.source)),
                            ("actor_2", actor_name(self.locale, world, incident.target)),
                            ("incident_damage", &format!("{}", incident.damage)),
                            (
                                "conditional",
                                if incident.fatal {
                                    tr("living.ascii.frame.fatal")
                                } else {
                                    ""
                                }
                            )
                        ]
                    )
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
            tr("living.ascii.frame.wasd_move_n_wait_f_id_attack")
        );
        let _ = writeln!(
            out,
            "{}",
            tr("living.ascii.frame.g_gather_h_deliver_v_save_o")
        );
        if !world.original_rules() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.m_practice_at_the_shrine_costs_1")
            );
        }
        if sites.is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.market_garden_hall_buy_sell_join_brew")
            );
        }
        if world.school().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.duty_carry_or_report_study_learn_form")
            );
        }
        if world.company().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.invite_id_travel_together_pack_id_give")
            );
        }
        if world.rivalry().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.v_envoy_accord_offer_1_herb_sponsor")
            );
        }
        if world.politics().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.q_hall_steward_pledge_talk_to_envoy")
            );
        }
        if world.traffic().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.move_toward_friendly_npc_to_pass_trafficlog")
            );
        }
        if world.relief().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.relieflog_witnessed_qinghe_crisis_and_reed_hall")
            );
        }
        if world.faction_agency_state().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.aidlog_witnessed_stonebank_surveys_physical_relief_and")
            );
        }
        if world.contracts().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr(
                    "living.ascii.frame.contractlog_witnessed_institutional_requests_bids_awards_and"
                )
            );
        }
        if world.contract_memory().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.accountlog_witnessed_site_audits_and_provider_reliability")
            );
        }
        if world.information().is_some() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.infolog_witnessed_testimony_and_institutional_disputes_you")
            );
        }
        if world.is_consequences() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.know_examine_ask_id_tell_id_event")
            );
        } else if world.is_player_agency() {
            let _ = writeln!(
                out,
                "{}",
                tr("living.ascii.frame.know_lists_what_you_know_examine_this")
            );
        }
        out
    }

    fn faction_status(&self) -> String {
        let tr = |key| living_text(self.locale, key);
        let Some(rivalry) = self.world.rivalry() else {
            return tr("living.ascii.faction_status.no_active_sect_dispute_in_this_campaign")
                .into();
        };
        let sites = Region::sites();
        if !self.debug_world
            && !self.world.player_sees(sites.market)
            && !self.world.player_sees(sites.hall)
        {
            return tr("living.ascii.faction_status.ask_in_willowford_to_learn_about_the").into();
        }
        let condition = if rivalry.at_peace(self.world.tick()) {
            tr("living.ascii.faction_status.truce")
        } else if rivalry.contested(self.world.tick()) {
            tr("living.ascii.faction_status.reed_hall_contested_teaching_suspended_and_herbs")
        } else {
            tr("living.ascii.faction_status.reed_hall_still_teaching")
        };
        let mut public = living_format(
            self.locale,
            "living.ascii.faction_status.template.condition_pressure_rivalry_pressure",
            &[
                ("condition", condition),
                ("rivalry_pressure", &format!("{}", rivalry.pressure)),
            ],
        );
        if let Some(politics) = self.world.politics() {
            let oath = if politics.allegiance == Allegiance::Stonebank {
                tr("living.ascii.faction_status.stonebank_member")
            } else {
                tr("living.ascii.faction_status.unaffiliated")
            };
            public.push_str(&format!(" {oath}."));
            public.push_str(&living_format(
                self.locale,
                "living.ascii.faction_status.template.hall_favor_politics_hall_favor_lodge",
                &[
                    ("politics_hall_favor", &format!("{}", politics.hall_favor)),
                    ("politics_lodge_favor", &format!("{}", politics.lodge_favor)),
                ],
            ));
        }
        if self.debug_world
            && let Some(agency) = self.world.faction_agency_state()
        {
            public.push_str(&living_format(
                self.locale,
                "living.ascii.faction_status.template.village_support_hall_stonebank_agency_public",
                &[(
                    "agency_public_standing",
                    &format!("{}", agency.public_standing),
                )],
            ));
        }
        if self.debug_world {
            let mut details = living_format(
                self.locale,
                "living.ascii.faction_status.template.public_lodge_treasury_rivalry_lodge_coins",
                &[
                    ("public", &public.to_string()),
                    ("rivalry_lodge_coins", &format!("{}", rivalry.lodge_coins)),
                ],
            );
            if let Some(politics) = self.world.politics() {
                details.push_str(&living_format(
                    self.locale,
                    "living.ascii.faction_status.template.hall_treasury_politics_hall_coins",
                    &[("politics_hall_coins", &format!("{}", politics.hall_coins))],
                ));
            }
            details
        } else {
            public
        }
    }

    pub(crate) fn company_line(&self, event: &CompanyEvent) -> String {
        let tr = |key| living_text(self.locale, key);
        let action = match event.kind {
            JourneyEvent::Joined => tr("living.ascii.company_line.joined_you"),
            JourneyEvent::Dismissed => tr("living.ascii.company_line.parted_ways_with_you"),
            JourneyEvent::LostContact => tr("living.ascii.company_line.lost_the_trail"),
            JourneyEvent::Distrust => tr("living.ascii.company_line.refused_to_continue_with_you"),
            JourneyEvent::Office => {
                tr("living.ascii.company_line.left_to_take_up_the_teaching_post")
            }
            JourneyEvent::Fallen => tr("living.ascii.company_line.fell"),
            JourneyEvent::PlayerFallen => tr("living.ascii.company_line.ended_the_journey"),
            JourneyEvent::FieldCare => {
                tr("living.ascii.company_line.used_a_packed_herb_to_treat_you")
            }
        };
        living_format(
            self.locale,
            "living.ascii.company_line.template.event_at_actor_action",
            &[
                ("event_at", &format!("{}", event.at)),
                ("actor", actor_name(self.locale, &self.world, event.actor)),
                ("action", action),
            ],
        )
    }

    fn traffic_line(&self, event: &sealed_sim::living::TrafficEvent) -> String {
        let action = match event.kind {
            TrafficEventKind::PlayerPass => living_text(
                self.locale,
                "living.ascii.traffic_line.exchanged_places_with",
            ),
            TrafficEventKind::NpcPass => {
                living_text(self.locale, "living.ascii.traffic_line.passed")
            }
        };
        living_format(
            self.locale,
            "living.ascii.traffic_line.template.event_at_actor_action_actor_2",
            &[
                ("event_at", &format!("{}", event.at)),
                ("actor", actor_name(self.locale, &self.world, event.mover)),
                ("action", action),
                ("actor_2", actor_name(self.locale, &self.world, event.other)),
                ("event_from_x", &format!("{}", event.from.x)),
                ("event_from_y", &format!("{}", event.from.y)),
                ("event_to_x", &format!("{}", event.to.x)),
                ("event_to_y", &format!("{}", event.to.y)),
            ],
        )
    }

    fn relief_line(&self, event: &sealed_sim::living::ReliefEvent) -> String {
        let action = match event.kind {
            ReliefEventKind::Observed => living_text(
                self.locale,
                "living.ascii.relief_line.noticed_an_urgent_medicine_shortage",
            ),
            ReliefEventKind::Reported => living_text(
                self.locale,
                "living.ascii.relief_line.asked_reed_hall_for_aid",
            ),
            ReliefEventKind::Purchased => living_text(
                self.locale,
                "living.ascii.relief_line.bought_an_herb_for_qinghe",
            ),
            ReliefEventKind::Redirected => living_text(
                self.locale,
                "living.ascii.relief_line.redirected_an_herb_to_qinghe",
            ),
            ReliefEventKind::Delivered => living_text(
                self.locale,
                "living.ascii.relief_line.delivered_medical_relief_to_qinghe",
            ),
            ReliefEventKind::Withdrawn => living_text(
                self.locale,
                "living.ascii.relief_line.stopped_a_no_longer_needed_relief_mission",
            ),
            ReliefEventKind::Failed => living_text(
                self.locale,
                "living.ascii.relief_line.lost_a_relief_mission",
            ),
        };
        living_format(
            self.locale,
            "living.ascii.relief_line.template.event_at_actor_action",
            &[
                ("event_at", &format!("{}", event.at)),
                ("actor", actor_name(self.locale, &self.world, event.actor)),
                ("action", action),
            ],
        )
    }

    fn faction_aid_line(&self, event: &sealed_sim::living::FactionEvent) -> String {
        let action = match event.kind {
            FactionEventKind::SurveyClear => living_text(
                self.locale,
                "living.ascii.faction_aid_line.found_qinghe_supplied",
            ),
            FactionEventKind::ShortageSeen => living_text(
                self.locale,
                "living.ascii.faction_aid_line.personally_witnessed_qinghe_s_medicine_shortage",
            ),
            FactionEventKind::Purchased => living_text(
                self.locale,
                "living.ascii.faction_aid_line.bought_a_real_herb_for_stonebank_relief",
            ),
            FactionEventKind::Redirected => living_text(
                self.locale,
                "living.ascii.faction_aid_line.redirected_personal_cargo_to_qinghe",
            ),
            FactionEventKind::Delivered => living_text(
                self.locale,
                "living.ascii.faction_aid_line.delivered_stonebank_medicine_to_qinghe",
            ),
            FactionEventKind::HallDelivered => living_text(
                self.locale,
                "living.ascii.faction_aid_line.delivered_reed_hall_medicine_to_qinghe",
            ),
            FactionEventKind::Withdrawn => living_text(
                self.locale,
                "living.ascii.faction_aid_line.left_the_aid_mission_to_honor_a",
            ),
            FactionEventKind::Failed => living_text(
                self.locale,
                "living.ascii.faction_aid_line.failed_to_finish_the_stonebank_relief_trip",
            ),
        };
        living_format(
            self.locale,
            "living.ascii.faction_aid_line.template.event_at_actor_action",
            &[
                ("event_at", &format!("{}", event.at)),
                ("actor", actor_name(self.locale, &self.world, event.actor)),
                ("action", action),
            ],
        )
    }

    fn contract_line(&self, event: &sealed_sim::living::ContractEvent) -> String {
        let objective = match event.need {
            Need::ShrineMedicine => {
                living_text(self.locale, "living.ascii.contract_line.qinghe_medicine")
            }
            Need::HallOfferings => {
                living_text(self.locale, "living.ascii.contract_line.reed_hall_supplies")
            }
        };
        let verb = match event.kind {
            ContractEventKind::Observed => {
                living_text(self.locale, "living.ascii.contract_line.noticed_a_need_for")
            }
            ContractEventKind::Offered => living_text(
                self.locale,
                "living.ascii.contract_line.offered_to_help_with",
            ),
            ContractEventKind::Declined => {
                living_text(self.locale, "living.ascii.contract_line.declined")
            }
            ContractEventKind::Awarded => living_text(
                self.locale,
                "living.ascii.contract_line.awarded_the_contract_for",
            ),
            ContractEventKind::Purchased => living_text(
                self.locale,
                "living.ascii.contract_line.bought_real_cargo_for",
            ),
            ContractEventKind::Redirected => living_text(
                self.locale,
                "living.ascii.contract_line.redirected_existing_cargo_to",
            ),
            ContractEventKind::Delivered => living_text(
                self.locale,
                "living.ascii.contract_line.physically_delivered",
            ),
            ContractEventKind::Failed => {
                living_text(self.locale, "living.ascii.contract_line.could_not_finish")
            }
            ContractEventKind::Withdrawn => {
                living_text(self.locale, "living.ascii.contract_line.withdrew_from")
            }
            ContractEventKind::ResolvedLocally => living_text(
                self.locale,
                "living.ascii.contract_line.resolved_locally_before_awarding",
            ),
        };
        living_format(
            self.locale,
            "living.ascii.contract_line.template.event_at_event_case_id_actor",
            &[
                ("event_at", &format!("{}", event.at)),
                ("event_case_id", &format!("{}", event.case_id)),
                ("actor", actor_name(self.locale, &self.world, event.actor)),
                ("verb", verb),
                ("objective", objective),
            ],
        )
    }

    fn account_line(&self, event: &sealed_sim::living::AuditEvent) -> String {
        let verdict = match event.finding {
            AuditFinding::Delivered => living_text(
                self.locale,
                "living.ascii.account_line.verified_a_signed_delivery_by",
            ),
            AuditFinding::Unfulfilled => living_text(
                self.locale,
                "living.ascii.account_line.found_no_delivery_by_the_deadline_from",
            ),
        };
        living_format(
            self.locale,
            "living.ascii.account_line.template.event_at_event_case_id_actor",
            &[
                ("event_at", &format!("{}", event.at)),
                ("event_case_id", &format!("{}", event.case_id)),
                (
                    "actor",
                    actor_name(self.locale, &self.world, event.requester),
                ),
                ("verdict", verdict),
                (
                    "actor_2",
                    actor_name(self.locale, &self.world, event.provider),
                ),
            ],
        )
    }

    fn info_line(&self, event: &sealed_sim::living::InfoLogEvent) -> String {
        let actor = actor_name(self.locale, &self.world, event.actor);
        let other = event
            .other
            .map(|id| actor_name(self.locale, &self.world, id))
            .unwrap_or_else(|| living_text(self.locale, "living.ascii.info_line.someone"));
        let phase = event.phase.map(|phase| match phase {
            DisputePhase::Open => living_text(self.locale, "living.ascii.info_line.opened"),
            DisputePhase::Investigating => {
                living_text(self.locale, "living.ascii.info_line.investigating")
            }
            DisputePhase::Confirmed => living_text(self.locale, "living.ascii.info_line.confirmed"),
            DisputePhase::Refuted => living_text(self.locale, "living.ascii.info_line.refuted"),
            DisputePhase::Suspended => living_text(self.locale, "living.ascii.info_line.suspended"),
            DisputePhase::Restored => living_text(self.locale, "living.ascii.info_line.restored"),
        });
        let tick = event.at.to_string();
        let id = event.event.0.to_string();
        match event.kind {
            InfoLogKind::Learned => living_text(
                self.locale,
                "living.ascii.info_line.t_tick_actor_learned_institutional_finding_id",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{id}", &id),
            InfoLogKind::DisputeOpened => living_text(
                self.locale,
                "living.ascii.info_line.t_tick_actor_verb_a_dispute_about",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace(
                "{verb}",
                phase.unwrap_or(living_text(self.locale, "living.ascii.info_line.opened")),
            )
            .replace("{other}", other),
            InfoLogKind::DisputeUpdated => living_text(
                self.locale,
                "living.ascii.info_line.t_tick_actor_marked_the_dispute_about",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{other}", other)
            .replace(
                "{status}",
                phase.unwrap_or(living_text(self.locale, "living.ascii.info_line.updated")),
            ),
            InfoLogKind::TestimonyQueued => living_text(
                self.locale,
                "living.ascii.info_line.t_tick_actor_began_briefing_other_about",
            )
            .replace("{tick}", &tick)
            .replace("{actor}", actor)
            .replace("{other}", other)
            .replace("{id}", &id),
        }
    }

    pub(crate) fn duty_hint(&self) -> Option<&'static str> {
        let tr = |key| living_text(self.locale, key);
        match self.world.school()?.duty {
            Duty::Carrying => Some(tr(
                "living.ascii.duty_hint.hall_parcel_use_duty_at_the_qinghe",
            )),
            Duty::Delivered => Some(tr(
                "living.ascii.duty_hint.hall_parcel_delivered_return_to_the_teacher",
            )),
            _ => None,
        }
    }

    pub(crate) fn hall_hint(&self) -> String {
        let tr = |key| living_text(self.locale, key);
        if self
            .world
            .politics()
            .is_some_and(|politics| politics.allegiance == Allegiance::Stonebank)
        {
            return tr("living.ascii.hall_hint.stonebank_oath_blocks_reed_hall_teaching_renounce")
                .into();
        }
        if self
            .world
            .rivalry()
            .is_some_and(|rivalry| rivalry.contested(self.world.tick()))
        {
            return tr("living.ascii.hall_hint.reed_hall_teaching_disrupted_offer_accord_to")
                .into();
        }
        let Some(school) = self.world.school() else {
            return tr("living.ascii.hall_hint.reed_hall_join_or_brew").into();
        };
        let Some(leader) = school.leader.and_then(|id| self.world.npc(id)) else {
            return tr("living.ascii.hall_hint.the_teaching_post_is_empty").into();
        };
        if self.world.is_night() || leader.position != Region::sites().hall {
            return tr("living.ascii.hall_hint.the_hall_s_teacher_is_away").into();
        }
        let prompt = match self.world.hall_rank() {
            None => tr("living.ascii.hall_hint.join_after_practicing_breath"),
            Some(HallRank::Apprentice) => {
                tr("living.ascii.hall_hint.duty_for_shrine_service_brew_for_a")
            }
            Some(HallRank::Disciple) if !school.form_learned => {
                tr("living.ascii.hall_hint.study_with_mastered_breath_and_2_herbs")
            }
            Some(HallRank::Disciple) if self.world.sect_hierarchy().is_some() => {
                tr("living.ascii.hall_hint.duty_for_senior_rank_brew_for_a")
            }
            Some(HallRank::Disciple) => tr("living.ascii.hall_hint.brew_a_remedy"),
            Some(HallRank::SeniorDisciple | HallRank::Elder) => {
                tr("living.ascii.hall_hint.allocate_for_hall_herb_duty_or_brew")
            }
        };
        if let Some(record) = school
            .history
            .last()
            .filter(|record| record.successor == Some(leader.id()))
        {
            return living_format(
                self.locale,
                "living.ascii.hall_hint.template.leader_definition_name_succeeded_actor_prompt",
                &[
                    ("leader_definition_name", leader.definition.name),
                    ("actor", actor_name(self.locale, &self.world, record.former)),
                    ("prompt", prompt),
                ],
            );
        }
        living_format(
            self.locale,
            "living.ascii.hall_hint.template.leader_definition_name_prompt",
            &[
                ("leader_definition_name", leader.definition.name),
                ("prompt", prompt),
            ],
        )
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
                return Err(living_text(locale, "living.save.foreign").into());
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
        return Err(living_text(locale, "living.load.foreign").into());
    }
    Ok(world)
}

fn load(path: &Path, locale: Locale) -> Result<World, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let bytes = persistence::read_limited(file, MAX_SAVE_BYTES)?;
    World::restore(&bytes).map_err(|error| match error {
        LoadError::LegacyCheckpoint => living_text(locale, "living.load.legacy").into(),
        _ => format!("{error:?}"),
    })
}

fn actor_name(locale: Locale, world: &World, id: ActorId) -> &'static str {
    if id == PLAYER {
        living_text(locale, "living.ascii.actor_name.you")
    } else {
        world.npc(id).map_or(
            living_text(locale, "living.ascii.actor_name.unknown"),
            |npc| npc.definition.name,
        )
    }
}

fn background_label(locale: Locale, background: Background) -> &'static str {
    match background {
        Background::Wanderer => living_text(locale, "living.background.wanderer"),
        Background::Herbalist => living_text(locale, "living.background.herbalist"),
        Background::Peddler => living_text(locale, "living.background.peddler"),
    }
}

fn goal_label(locale: Locale, goal: Goal) -> &'static str {
    let key = match goal {
        Goal::Idle => "living.ascii.goal_label.idle",
        Goal::Rest => "living.ascii.goal_label.resting_or_heading_home",
        Goal::Patrol => "living.ascii.goal_label.patrolling",
        Goal::TendClinic => "living.ascii.goal_label.staffing_the_clinic",
        Goal::CollectHerbs => "living.ascii.goal_label.collecting_herbs",
        Goal::DeliverHerbs => "living.ascii.goal_label.hauling_herbs_to_the_shrine",
        Goal::SeekCare => "living.ascii.goal_label.seeking_care",
        Goal::Treat(_) => "living.ascii.goal_label.attending_to_a_patient",
        Goal::Investigate(_) => "living.ascii.goal_label.investigating_a_disturbance",
        Goal::InvestigateBody(_) => "living.ascii.goal_label.checking_a_reported_body",
        Goal::RecoverBody(_) => "living.ascii.goal_label.recovering_abandoned_cargo",
        Goal::ReturnRecoveredCargo => "living.ascii.goal_label.returning_recovered_herbs",
        Goal::ClaimLot(_) => "living.ascii.goal_label.seeking_a_relic_lot",
        Goal::DeliverLot(_) => "living.ascii.goal_label.carrying_a_relic_to_the_sect",
        Goal::MartialPractice(_) => "living.ascii.goal_label.training_with_a_mentor",
        Goal::MartialRest => "living.ascii.goal_label.recovering_from_training",
        Goal::MeridianDraw => "living.ascii.goal_label.drawing_qi_from_an_offering",
        Goal::MeridianCirculate => "living.ascii.goal_label.circulating_qi",
        Goal::MeridianCare => "living.ascii.goal_label.seeking_meridian_care",
        Goal::SeekWoundCare => "living.ascii.goal_label.seeking_wound_care",
        Goal::GatherForCare => "living.ascii.goal_label.working_to_afford_treatment",
        Goal::SellForCare => "living.ascii.goal_label.selling_work_herbs_for_treatment",
        Goal::SectDuty => "living.ascii.goal_label.carrying_out_reed_hall_service",
        Goal::GatherFrontier => "living.expansion.goal.gather",
        Goal::StockFrontierDepot => "living.expansion.goal.stock",
        Goal::CollectFrontierFreight => "living.expansion.goal.collect",
        Goal::DeliverFrontierFreight => "living.expansion.goal.deliver",
        Goal::Report { .. } => "living.ascii.goal_label.seeking_someone_to_report_to",
        Goal::Retreat => "living.ascii.goal_label.retreating",
        Goal::Retaliate => "living.ascii.goal_label.retaliating",
        Goal::StockGarden => "living.ascii.goal_label.collecting_garden_herbs",
        Goal::StockMarket => "living.ascii.goal_label.stocking_willowford_market",
        Goal::CollectFreight => "living.ascii.goal_label.picking_up_qinghe_s_order",
        Goal::DeliverFreight => "living.ascii.goal_label.carrying_qinghe_s_order",
        Goal::Follow(_) => "living.ascii.goal_label.following_the_trail",
        Goal::FieldCare => "living.ascii.goal_label.giving_field_care",
        Goal::TeachAtHall => "living.ascii.goal_label.teaching_at_reed_hall",
        Goal::BuyForLodge => "living.ascii.goal_label.buying_herbs_for_stonebank_lodge",
        Goal::PetitionHall => "living.ascii.goal_label.pressing_stonebank_s_claim",
        Goal::BuyForHall => "living.ascii.goal_label.buying_herbs_for_reed_hall",
        Goal::SupplyHall => "living.ascii.goal_label.delivering_herbs_to_reed_hall",
        Goal::RequestRelief => "living.ascii.goal_label.carrying_a_plea_for_medical_relief",
        Goal::BuyRelief => "living.ascii.goal_label.procuring_crisis_medicine",
        Goal::DeliverRelief => "living.ascii.goal_label.carrying_emergency_medicine",
        Goal::CampRest => "living.ascii.goal_label.resting_on_the_road",
        Goal::SurveyQinghe => "living.ascii.goal_label.surveying_qinghe_for_stonebank",
        Goal::BuyLodgeAid => "living.ascii.goal_label.buying_stonebank_relief_medicine",
        Goal::DeliverLodgeAid => "living.ascii.goal_label.carrying_stonebank_relief_medicine",
        Goal::NegotiateContract(_) => {
            "living.ascii.goal_label.negotiating_an_institutional_contract"
        }
        Goal::ProcureContract(_) => "living.ascii.goal_label.procuring_contract_supplies",
        Goal::FulfillContract(_) => "living.ascii.goal_label.delivering_contract_supplies",
        Goal::AuditContract(_) => "living.ascii.goal_label.inspecting_the_delivery_site",
        Goal::DeliverTestimony { .. } => "living.ascii.goal_label.carrying_institutional_testimony",
        Goal::InvestigateDispute(_) => {
            "living.ascii.goal_label.investigating_an_institutional_dispute"
        }
    };
    living_text(locale, key)
}

pub(crate) fn outcome_label(locale: Locale, world: &World, outcome: Outcome) -> String {
    let tr = |key| living_text(locale, key);
    let key = match outcome {
        Outcome::JoinedCompany(id) | Outcome::Parted(id) | Outcome::Provisioned(id) => {
            let action = match outcome {
                Outcome::JoinedCompany(_) => {
                    tr("living.ascii.outcome_label.agreed_to_travel_with_you")
                }
                Outcome::Parted(_) => tr("living.ascii.outcome_label.parted_ways_with_you"),
                _ => tr("living.ascii.outcome_label.accepted_a_medicinal_herb"),
            };
            return living_format(
                locale,
                "living.ascii.outcome_label.template.actor_action",
                &[("actor", actor_name(locale, world, id)), ("action", action)],
            );
        }
        Outcome::Waited => "living.ascii.outcome_label.time_passed",
        Outcome::Moved => "living.ascii.outcome_label.you_moved",
        Outcome::Spoke(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_spoke_with_actor",
                &[("actor", actor_name(locale, world, id))],
            );
        }
        Outcome::Gathered => "living.ascii.outcome_label.you_gathered_an_herb",
        Outcome::GatheredPair => "living.ascii.outcome_label.you_gathered_two_herbs",
        Outcome::DutyAccepted => "living.ascii.outcome_label.you_took_one_herb_from_the_hall",
        Outcome::DutyDelivered => "living.ascii.outcome_label.the_hall_s_herb_entered_shrine_stock",
        Outcome::DutyReported => {
            if world.hall_rank() == Some(HallRank::SeniorDisciple) {
                "living.ascii.outcome_label.the_teacher_recorded_your_merit_you_stand"
            } else {
                "living.ascii.outcome_label.the_teacher_recognized_you_as_a_disciple"
            }
        }
        Outcome::FormLearned => "living.ascii.outcome_label.you_learned_reed_strike",
        Outcome::LodgeAccord { until } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.stonebank_agreed_to_cease_petitions_through",
                &[("until", &format!("{}", until))],
            );
        }
        Outcome::HallSupported => {
            "living.ascii.outcome_label.your_herb_strengthened_reed_hall_s_standing"
        }
        Outcome::LodgePledged => {
            "living.ascii.outcome_label.you_pledged_to_stonebank_and_left_reed"
        }
        Outcome::LodgeAided => {
            "living.ascii.outcome_label.you_provisioned_stonebank_and_strengthened_its_claim"
        }
        Outcome::LodgeRenounced => {
            "living.ascii.outcome_label.you_renounced_stonebank_at_reed_hall"
        }
        Outcome::Learned(event) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_examined_evidence_event_0",
                &[("event_0", &format!("{}", event.0))],
            );
        }
        Outcome::Briefed { from, event } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.actor_will_tell_you_about_finding",
                &[
                    ("actor", actor_name(locale, world, from)),
                    ("event_0", &format!("{}", event.0)),
                ],
            );
        }
        Outcome::Testified { to, event } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_offered_finding_event_0_actor",
                &[
                    ("event_0", &format!("{}", event.0)),
                    ("actor", actor_name(locale, world, to)),
                ],
            );
        }
        Outcome::Took { from, herbs } => {
            return living_plural(
                locale,
                "living.ascii.outcome_label.template.you_took_herbs_herbs_from_actor.one",
                "living.ascii.outcome_label.template.you_took_herbs_herbs_from_actor",
                u64::from(herbs),
                &[
                    ("herbs", &herbs.to_string()),
                    ("actor", actor_name(locale, world, from)),
                ],
            );
        }
        Outcome::BodyReported { to, body } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_reported_actor_to_actor_2",
                &[
                    ("actor", actor_name(locale, world, body)),
                    ("actor_2", actor_name(locale, world, to)),
                ],
            );
        }
        Outcome::LotClaimed(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_claimed_lot_id",
                &[("id", &format!("{}", id))],
            );
        }
        Outcome::LotAttuned(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_attuned_lot_id",
                &[("id", &format!("{}", id))],
            );
        }
        Outcome::ChannelStruck(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_channeled_a_strike_against_actor",
                &[("actor", actor_name(locale, world, id))],
            );
        }
        Outcome::MartialDrilled { teacher, learned } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_drilled_with_actor_conditional",
                &[
                    ("actor", actor_name(locale, world, teacher)),
                    (
                        "conditional",
                        if learned {
                            tr("living.ascii.outcome_label.and_learned_a_form")
                        } else {
                            ""
                        },
                    ),
                ],
            );
        }
        Outcome::TechniqueStruck { target, form } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.form_you_struck_actor",
                &[
                    ("form", &format!("{:?}", form)),
                    ("actor", actor_name(locale, world, target)),
                ],
            );
        }
        Outcome::QiDrawn(qi) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.qi",
                &[("qi", &format!("{}", qi))],
            );
        }
        Outcome::QiCirculated { channels, blockage } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.channels_blockage",
                &[
                    ("channels", &format!("{}", channels)),
                    ("blockage", &format!("{}", blockage)),
                ],
            );
        }
        Outcome::MeridiansMended => "living.ascii.outcome_label.your_meridians_were_treated",
        Outcome::WoundTreated => {
            "living.ascii.outcome_label.one_wound_was_treated_using_shrine_medicine"
        }
        Outcome::HallAllocationClaimed => {
            "living.ascii.outcome_label.you_claimed_one_physical_herb_from_reed"
        }
        Outcome::LotSold { to, id, coins } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_sold_lot_id_to_actor",
                &[
                    ("id", &format!("{}", id)),
                    ("actor", actor_name(locale, world, to)),
                    ("coins", &format!("{}", coins)),
                ],
            );
        }
        Outcome::QiCentered { total } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.total_max_qi",
                &[
                    ("total", &format!("{}", total)),
                    ("max_qi", &format!("{}", MAX_QI)),
                ],
            );
        }
        Outcome::FormStruck(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_used_reed_strike_on_actor",
                &[("actor", actor_name(locale, world, id))],
            );
        }
        Outcome::Delivered | Outcome::Contribution { witnesses: 0 } => {
            "living.ascii.outcome_label.you_supplied_the_shrine_with_an_herb"
        }
        Outcome::Contribution { witnesses } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_supplied_a_herb_during_a",
                &[("witnesses", &format!("{}", witnesses))],
            );
        }
        Outcome::Practiced { progress } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_spent_one_herb_on_breath",
                &[
                    ("progress", &format!("{}", progress)),
                    ("breath_mastery", &format!("{}", BREATH_MASTERY)),
                ],
            );
        }
        Outcome::Struck(id) => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_struck_2_actor",
                &[("actor", actor_name(locale, world, id))],
            );
        }
        Outcome::Guarded => "living.ascii.outcome_label.you_guarded_this_turn",
        Outcome::BackgroundChosen(background) => {
            return background_label(locale, background).into();
        }
        Outcome::Traded { bought, coins } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.conditional_coins",
                &[
                    (
                        "conditional",
                        if bought {
                            tr("living.ascii.outcome_label.you_bought_one_herb_for")
                        } else {
                            tr("living.ascii.outcome_label.you_sold_one_herb_for")
                        },
                    ),
                    ("coins", &format!("{}", coins)),
                ],
            );
        }
        Outcome::Enrolled => "living.ascii.outcome_label.you_became_a_reed_hall_apprentice",
        Outcome::Brewed => "living.ascii.outcome_label.you_brewed_a_remedy_from_two_herbs",
        Outcome::UsedRemedy { healed } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.you_took_a_remedy_healed",
                &[("healed", &format!("{}", healed))],
            );
        }
        Outcome::Treated { by, healed } => {
            return living_format(
                locale,
                "living.ascii.outcome_label.template.actor_treated_you_healed",
                &[
                    ("actor", actor_name(locale, world, by)),
                    ("healed", &format!("{}", healed)),
                ],
            );
        }
        Outcome::Rejected(reason) => match reason {
            Rejection::Dead => "living.ascii.outcome_label.you_are_dead_you_can_still_wait",
            Rejection::Impassable => "living.ascii.outcome_label.the_way_is_blocked",
            Rejection::Occupied => "living.ascii.outcome_label.someone_is_in_the_way",
            Rejection::UnknownActor => "living.ascii.outcome_label.that_npc_is_unavailable",
            Rejection::OutOfReach => "living.ascii.outcome_label.you_need_to_be_within_one_tile",
            Rejection::Refused => "living.ascii.outcome_label.the_npc_refuses_to_help_you",
            Rejection::HealerAway => "living.ascii.outcome_label.ask_the_healer_at_the_clinic",
            Rejection::Empty => "living.ascii.outcome_label.the_supply_is_empty",
            Rejection::NoEvidence => {
                "living.ascii.outcome_label.nothing_here_records_an_institutional_finding"
            }
            Rejection::Overtrained => "living.ascii.outcome_label.too_strained_wait_and_recover",
            Rejection::MeridiansBlocked => {
                "living.ascii.outcome_label.blocked_meridians_require_a_healer"
            }
            Rejection::Hobbled => "living.ascii.outcome_label.your_leg_wound_slows_you_this_turn",
            Rejection::NoCargo => "living.ascii.outcome_label.you_need_more_herbs",
            Rejection::PracticeComplete => {
                "living.ascii.outcome_label.you_already_mastered_this_practice"
            }
            Rejection::BackgroundLocked => {
                "living.ascii.outcome_label.choose_a_background_before_your_first_turn"
            }
            Rejection::MarketClosed => "living.ascii.outcome_label.the_market_opens_at_dawn",
            Rejection::NotEnoughCoins => {
                "living.ascii.outcome_label.there_are_insufficient_coins_for_the_trade"
            }
            Rejection::HallClosed => "living.ascii.outcome_label.come_back_when_the_mentor_is_at",
            Rejection::AlreadyEnrolled => {
                "living.ascii.outcome_label.you_are_already_an_apprentice"
            }
            Rejection::PracticeRequired => {
                "living.ascii.outcome_label.practice_breath_at_the_qinghe_shrine_first"
            }
            Rejection::MembershipRequired => "living.ascii.outcome_label.join_reed_hall_first",
            Rejection::NoRemedy => "living.ascii.outcome_label.you_have_no_remedies",
            Rejection::Uninjured => "living.ascii.outcome_label.save_your_remedy_for_a_wound",
            Rejection::DutyComplete => {
                "living.ascii.outcome_label.you_already_reported_the_shrine_duty"
            }
            Rejection::RankRequired => {
                if world.sect_hierarchy().is_some() && world.hall_rank() == Some(HallRank::Disciple)
                {
                    "living.ascii.outcome_label.earn_more_merit_to_reach_senior_disciple"
                } else {
                    "living.ascii.outcome_label.complete_your_shrine_duty_and_become_a"
                }
            }
            Rejection::FormAlreadyKnown => {
                "living.ascii.outcome_label.you_already_know_reed_strike"
            }
            Rejection::FormRequired => {
                "living.ascii.outcome_label.learn_reed_strike_at_the_hall_first"
            }
            Rejection::QiFull => "living.ascii.outcome_label.your_qi_is_full",
            Rejection::IntroductionRequired => {
                "living.ascii.outcome_label.talk_to_each_other_first"
            }
            Rejection::BoundToDuty => {
                "living.ascii.outcome_label.this_person_stays_with_their_duties"
            }
            Rejection::CompanyFull => {
                "living.ascii.outcome_label.you_already_have_a_travelling_companion"
            }
            Rejection::NotCompanion => "living.ascii.outcome_label.you_are_not_travelling_together",
            Rejection::PackFull => {
                "living.ascii.outcome_label.the_medicine_pouch_holds_two_herbs_and"
            }
            Rejection::SchoolContested => {
                "living.ascii.outcome_label.stonebank_s_claim_has_suspended_the_teacher"
            }
            Rejection::OathConflict => {
                "living.ascii.outcome_label.your_current_allegiance_prevents_that_action"
            }
            Rejection::FactionTrustRequired => {
                "living.ascii.outcome_label.earn_the_faction_s_trust_through_material"
            }
            Rejection::InsufficientQi => {
                return living_format(
                    locale,
                    "living.ascii.outcome_label.template.reed_strike_needs_form_qi_cost",
                    &[("form_qi_cost", &format!("{}", FORM_QI_COST))],
                );
            }
        },
    };
    living_text(locale, key).into()
}

pub(crate) fn run(debug_world: bool) {
    run_campaign(debug_world, 26);
}

fn hall_rank_label(locale: Locale, rank: HallRank) -> &'static str {
    let key = match rank {
        HallRank::Apprentice => "living.ascii.hall_rank_label.reed_hall_apprentice",
        HallRank::Disciple => "living.ascii.hall_rank_label.reed_hall_disciple",
        HallRank::SeniorDisciple => "living.ascii.hall_rank_label.reed_hall_senior_disciple",
        HallRank::Elder => "living.ascii.hall_rank_label.reed_hall_elder",
    };
    living_text(locale, key)
}

fn decision_reason_label(locale: Locale, reason: DecisionReason) -> &'static str {
    let key = match reason {
        DecisionReason::ImmediateDanger => "living.ascii.decision_reason_label.immediate_danger",
        DecisionReason::PhysicalRecovery => "living.ascii.decision_reason_label.physical_recovery",
        DecisionReason::TreatmentFunds => "living.ascii.decision_reason_label.treatment_costs",
        DecisionReason::MedicalDuty => "living.ascii.decision_reason_label.medical_duty",
        DecisionReason::Cultivation => "living.ascii.decision_reason_label.cultivation",
        DecisionReason::MartialAmbition => "living.ascii.decision_reason_label.martial_ambition",
        DecisionReason::InstitutionalObligation => {
            "living.ascii.decision_reason_label.institutional_obligation"
        }
        DecisionReason::KnownOpportunity => {
            "living.ascii.decision_reason_label.a_known_opportunity"
        }
        DecisionReason::Fatigue => "living.ascii.decision_reason_label.fatigue",
        DecisionReason::Livelihood => "living.ascii.decision_reason_label.livelihood",
        DecisionReason::Routine => "living.ascii.decision_reason_label.routine",
        DecisionReason::ResumingAmbition => {
            "living.ascii.decision_reason_label.resuming_an_interrupted_pursuit"
        }
    };
    living_text(locale, key)
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
        27 => "checkpoint.c40",
        28 => "checkpoint.c41",
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
        27 => World::generated_riverlands(0),
        28 => World::expanded_riverlands(0),
        _ => unreachable!("unsupported campaign revision"),
    };
    session.debug_world = debug_world;
    session
}

pub(crate) fn expanded_session(seed: u32, debug_world: bool) -> Session {
    let mut session = campaign_session(debug_world, 28);
    if seed != 0 {
        session.world = World::expanded_riverlands(seed);
    }
    session
}
pub(crate) fn run_expanded_campaign(seed: u32, debug_world: bool) {
    run_session(expanded_session(seed, debug_world));
}
pub(crate) fn generated_session(seed: u32, debug_world: bool) -> Session {
    let mut session = campaign_session(debug_world, 27);
    if seed != 0 {
        session.world = World::generated_riverlands(seed);
    }
    session
}

pub(crate) fn run_generated_campaign(seed: u32, debug_world: bool) {
    run_session(generated_session(seed, debug_world));
}

fn run_campaign(debug_world: bool, revision: u16) {
    run_session(campaign_session(debug_world, revision));
}

fn run_session(mut session: Session) {
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
    fn changing_language_drops_stale_notice_without_mutating_checkpoint() {
        let mut session = campaign_session(false, 26);
        session.input("help");
        assert!(!session.notice.is_empty());
        let before = session.world.checkpoint();
        session.input("lang id");
        assert_eq!(session.locale, Locale::Indonesian);
        assert!(session.notice.is_empty());
        assert_eq!(session.world.checkpoint(), before);
        session.input("help");
        assert!(session.notice.contains("Balai Alang-alang"));
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
