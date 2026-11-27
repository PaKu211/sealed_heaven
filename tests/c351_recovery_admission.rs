use sealed_sim::living::{ActorId, Background, Direction, EventId, Intent, StepError, World};

#[test]
fn packed_testimony_rejects_lossy_identity_without_advancing_time() {
    for seed in [0, 1] {
        let mut world = World::sect_hierarchy_campaign(seed);
        for intent in [
            Intent::Testify {
                to: ActorId::new(257),
                event: EventId(0),
            },
            Intent::Testify {
                to: ActorId::new(1),
                event: EventId(0x0100_0000),
            },
        ] {
            let before = world.checkpoint();
            assert_eq!(
                world.step(world.tick(), intent),
                Err(StepError::UnrecordableIntent)
            );
            assert_eq!(world.checkpoint(), before);
        }
        world.step(world.tick(), Intent::Wait).unwrap();
        assert_eq!(
            World::restore(&world.checkpoint()).unwrap().checkpoint(),
            world.checkpoint()
        );
    }
}

#[test]
fn historical_campaign_rejects_unsupported_information_intents_before_commit() {
    for mut world in [
        World::institutional_agency(0),
        World::contract_accountability(0),
        World::information_disputes(0),
    ] {
        for intent in [
            Intent::Ask(ActorId::new(1)),
            Intent::Examine,
            Intent::Testify {
                to: ActorId::new(1),
                event: EventId(0),
            },
        ] {
            let before = world.checkpoint();
            assert_eq!(
                world.step(world.tick(), intent),
                Err(StepError::UnrecordableIntent)
            );
            assert_eq!(world.checkpoint(), before);
        }
    }
}

// This covers the public admission/codec contract rather than relying on the
// UI parser or private state fixtures. A normal gameplay rejection may consume
// a turn; a structural rejection must not mutate anything.
#[test]
fn every_admitted_intent_roundtrips_across_released_seeded_campaigns() {
    let campaigns: [fn(u32) -> World; 31] = [
        World::riverlands,
        World::reed_hall,
        World::road_company,
        World::sect_rivalry,
        World::sect_politics,
        World::traffic_courtesy,
        World::living_agency,
        World::faction_agency,
        World::institutional_agency,
        World::contract_accountability,
        World::information_disputes,
        World::player_agency,
        World::consequences,
        World::violence_logistics,
        World::player_reporting,
        World::physical_recovery,
        World::contested_resources,
        World::sect_cultivation,
        World::martial_mentorship,
        World::meridian_cultivation,
        World::persistent_injury,
        World::autonomous_agency,
        World::sect_hierarchy_campaign,
        World::duty_coherence_campaign,
        World::generated_riverlands,
        World::expanded_riverlands,
        World::regional_knowledge_campaign,
        World::exact_lod_campaign,
        World::settlement_production_campaign,
        World::road_history_campaign,
        World::emergent_requests_campaign,
    ];
    let actor = ActorId::new(1);
    let intents = [
        Intent::Wait,
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::South),
        Intent::Move(Direction::West),
        Intent::Talk(actor),
        Intent::Gather,
        Intent::Deliver,
        Intent::Attack(actor),
        Intent::Guard,
        Intent::Practice,
        Intent::ChooseBackground(Background::Wanderer),
        Intent::ChooseBackground(Background::Herbalist),
        Intent::ChooseBackground(Background::Peddler),
        Intent::BuyHerb,
        Intent::SellHerb,
        Intent::JoinHall,
        Intent::Brew,
        Intent::UseRemedy,
        Intent::HallDuty,
        Intent::StudyForm,
        Intent::CenterQi,
        Intent::ReedStrike(actor),
        Intent::Invite(actor),
        Intent::Provision(actor),
        Intent::Part(actor),
        Intent::OfferAccord,
        Intent::SupportHall,
        Intent::PledgeLodge,
        Intent::AidLodge,
        Intent::RenounceLodge,
        Intent::Ask(actor),
        Intent::Examine,
        Intent::Testify {
            to: actor,
            event: EventId(0),
        },
        Intent::Testify {
            to: ActorId::new(255),
            event: EventId(0x00ff_ffff),
        },
        Intent::Testify {
            to: ActorId::new(256),
            event: EventId(0),
        },
        Intent::Testify {
            to: actor,
            event: EventId(0x0100_0000),
        },
        Intent::Take(actor),
        Intent::ReportBody {
            to: actor,
            body: ActorId::new(32),
        },
        Intent::ReportBody {
            to: ActorId::new(256),
            body: ActorId::new(32),
        },
        Intent::ClaimLot(1),
        Intent::OfferLot { to: actor, id: 1 },
        Intent::OfferLot {
            to: actor,
            id: 0x0100_0000,
        },
        Intent::AttuneLot(1),
        Intent::ChannelStrike(actor),
        Intent::Drill(actor),
        Intent::TechniqueStrike(actor),
        Intent::DrawQi,
        Intent::CirculateQi,
        Intent::MendMeridians,
        Intent::TreatWounds,
        Intent::ClaimHallAllocation,
        Intent::RecoverCache(1),
        Intent::EntrustCache(2),
        Intent::AcceptRequest(1),
        Intent::DeclineRequest(2),
        Intent::AbandonRequest(u32::MAX),
    ];
    for seed in [0, 1] {
        for make in campaigns {
            for intent in intents {
                let mut world = make(seed);
                let revision = world.checkpoint_revision();
                let before = world.checkpoint();
                match world.step(world.tick(), intent) {
                    Err(_) => assert_eq!(world.checkpoint(), before),
                    Ok(_) => {
                        let saved = world.checkpoint();
                        let mut restored = World::restore(&saved).unwrap_or_else(|error| {
                            panic!("revision {revision}, seed {seed}, intent {intent:?}: {error:?}")
                        });
                        assert_eq!(restored.checkpoint(), saved);
                        assert_eq!(
                            world.step(world.tick(), Intent::Wait),
                            restored.step(restored.tick(), Intent::Wait)
                        );
                        assert_eq!(world.checkpoint(), restored.checkpoint());
                    }
                }
            }
        }
    }
}

#[test]
fn canonical_intent_records_keep_the_historical_five_byte_layout() {
    let mut world = World::sect_hierarchy_campaign(0);
    let intents: [(Intent, [u8; 5]); 3] = [
        (Intent::Wait, [0, 0, 0, 0, 0]),
        (Intent::Move(Direction::East), [1, 1, 0, 0, 0]),
        (
            Intent::Testify {
                to: ActorId::new(255),
                event: EventId(0x00ff_ffff),
            },
            [28, 255, 255, 255, 255],
        ),
    ];
    for (index, (intent, expected)) in intents.into_iter().enumerate() {
        world.step(world.tick(), intent).unwrap();
        let checkpoint = world.checkpoint();
        let offset = 54 + index * 5;
        assert_eq!(&checkpoint[offset..offset + 5], &expected);
        assert_eq!(
            World::restore(&checkpoint).unwrap().checkpoint(),
            checkpoint
        );
    }
}
