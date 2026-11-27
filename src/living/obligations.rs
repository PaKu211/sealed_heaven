//! C5.2: promises created by actual coordinated shipments. World outcomes
//! and situated acknowledgement are distinct. No omniscient blame, money,
//! debt, cargo identity, or permanent blacklist is introduced.
use super::{
    ActorId, Goal, Npc, Position, World,
    cooperation::{self, Route, Team},
    navigation::in_reach,
};
use std::collections::{BTreeMap, VecDeque};

pub const REVISION: u16 = 39;
pub const VERSION: u16 = 1;
pub const REPORT_LIFETIME: u64 = 480;
pub const RETRY_INTERVAL: u64 = 96;
pub const HISTORY_CAP: usize = 128;
pub const OBSERVED_CAP: usize = 48;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ResultKind {
    Fulfilled,
    ChangedCargo,
    CarrierLost,
    Released,
    Expired,
}
impl ResultKind {
    fn from_outcome(o: cooperation::Outcome) -> Self {
        match o {
            cooperation::Outcome::Delivered => Self::Fulfilled,
            cooperation::Outcome::CargoChanged => Self::ChangedCargo,
            cooperation::Outcome::CarrierLost => Self::CarrierLost,
            cooperation::Outcome::Withdrawn => Self::Released,
            cooperation::Outcome::Expired => Self::Expired,
            cooperation::Outcome::Dispatch => unreachable!("dispatch does not close a promise"),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub team: Team,
    pub at: u64,
    pub place: Position,
    pub result: ResultKind,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Evidence {
    Conversation,
    Register,
    Relayed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Acknowledgement {
    pub resolution: Resolution,
    pub at: u64,
    pub place: Position,
    pub evidence: Evidence,
    pub regard_change: i8,
}
/// Public historical acknowledgement, with no private trust delta or account.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Witnessed {
    pub id: u32,
    pub grower: ActorId,
    pub carrier: ActorId,
    pub result: ResultKind,
    pub at: u64,
    pub place: Position,
    pub evidence: Evidence,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Reliability {
    pub fulfilled: u32,
    pub released: u32,
    pub disrupted: u32,
    pub expired: u32,
    pub last_at: u64,
    pub retry_at: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Opened(Team),
    Resolved(Resolution),
    Acknowledged(Acknowledgement),
    Unreported { resolution: Resolution, at: u64 },
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Obligations {
    // One pending report per carrier; subsequent coordinated jobs wait for
    // the known report or its finite expiry. Normal commerce is unrestricted.
    pending: BTreeMap<ActorId, Resolution>,
    reliability: BTreeMap<(ActorId, ActorId), Reliability>,
    history: VecDeque<Event>,
    observed: VecDeque<Witnessed>,
    opened: u32,
    resolved: u32,
    acknowledged: u32,
    unreported: u32,
}
impl Obligations {
    pub fn pending(&self) -> &BTreeMap<ActorId, Resolution> {
        &self.pending
    }
    pub fn reliability(&self, grower: ActorId, carrier: ActorId) -> Reliability {
        self.reliability
            .get(&(grower, carrier))
            .copied()
            .unwrap_or_default()
    }
    pub fn history(&self) -> &VecDeque<Event> {
        &self.history
    }
    pub fn observed(&self) -> &VecDeque<Witnessed> {
        &self.observed
    }
    pub fn counts(&self) -> (u32, u32, u32, u32) {
        (
            self.opened,
            self.resolved,
            self.acknowledged,
            self.unreported,
        )
    }
    fn push(&mut self, event: Event) {
        if self.history.len() == HISTORY_CAP {
            self.history.pop_front();
        }
        self.history.push_back(event);
    }
    fn open(&mut self, team: Team) {
        self.opened += 1;
        self.push(Event::Opened(team));
    }
    fn resolve(&mut self, resolution: Resolution) {
        self.resolved += 1;
        self.push(Event::Resolved(resolution));
        if resolution.result == ResultKind::CarrierLost {
            self.unreported += 1;
            self.push(Event::Unreported {
                resolution,
                at: resolution.at,
            });
        } else {
            assert!(
                self.pending
                    .insert(resolution.team.carrier, resolution)
                    .is_none()
            );
        }
    }
    fn acknowledge(&mut self, ack: Acknowledgement, seen: bool) {
        let t = ack.resolution.team;
        assert_eq!(self.pending.remove(&t.carrier), Some(ack.resolution));
        let r = self.reliability.entry((t.grower, t.carrier)).or_default();
        match ack.resolution.result {
            ResultKind::Fulfilled => {
                r.fulfilled += 1;
                r.retry_at = 0;
            }
            ResultKind::Released => {
                r.released += 1;
                r.retry_at = ack.at + RETRY_INTERVAL;
            }
            ResultKind::ChangedCargo => r.disrupted += 1,
            ResultKind::Expired => r.expired += 1,
            ResultKind::CarrierLost => unreachable!("a dead carrier cannot report"),
        }
        r.last_at = ack.at;
        self.acknowledged += 1;
        self.push(Event::Acknowledged(ack));
        if seen {
            if self.observed.len() == OBSERVED_CAP {
                self.observed.pop_front();
            }
            self.observed.push_back(Witnessed {
                id: t.id,
                grower: t.grower,
                carrier: t.carrier,
                result: ack.resolution.result,
                at: ack.at,
                place: ack.place,
                evidence: ack.evidence,
            });
        }
    }
    fn forget(&mut self, carrier: ActorId, at: u64) {
        if let Some(resolution) = self.pending.remove(&carrier) {
            self.unreported += 1;
            self.push(Event::Unreported { resolution, at });
        }
    }
    pub(super) fn canonical_bytes(&self) -> Vec<u8> {
        fn pos(out: &mut Vec<u8>, p: Position) {
            out.extend(p.x.to_le_bytes());
            out.extend(p.y.to_le_bytes());
        }
        fn team(out: &mut Vec<u8>, t: Team) {
            out.extend(t.id.to_le_bytes());
            out.push(t.route as u8);
            out.extend(t.grower.get().to_le_bytes());
            out.extend(t.carrier.get().to_le_bytes());
            out.extend(t.opened.to_le_bytes());
            out.extend(t.deadline.to_le_bytes());
            out.extend(t.contributed.to_le_bytes());
            out.extend(t.shipment.to_le_bytes());
            out.push(u8::from(t.opening_seen));
        }
        fn resolution(out: &mut Vec<u8>, r: Resolution) {
            team(out, r.team);
            out.extend(r.at.to_le_bytes());
            pos(out, r.place);
            out.push(r.result as u8);
        }
        let mut out = b"SH:C52:OBLIGATIONS:v1\0".to_vec();
        out.extend(VERSION.to_le_bytes());
        for n in [
            self.opened,
            self.resolved,
            self.acknowledged,
            self.unreported,
            self.pending.len() as u32,
        ] {
            out.extend(n.to_le_bytes());
        }
        for r in self.pending.values() {
            resolution(&mut out, *r);
        }
        out.extend((self.reliability.len() as u32).to_le_bytes());
        for (&(a, b), r) in &self.reliability {
            out.extend(a.get().to_le_bytes());
            out.extend(b.get().to_le_bytes());
            for n in [r.fulfilled, r.released, r.disrupted, r.expired] {
                out.extend(n.to_le_bytes());
            }
            out.extend(r.last_at.to_le_bytes());
            out.extend(r.retry_at.to_le_bytes());
        }
        out.extend((self.history.len() as u32).to_le_bytes());
        for e in &self.history {
            match *e {
                Event::Opened(t) => {
                    out.push(0);
                    team(&mut out, t);
                }
                Event::Resolved(r) => {
                    out.push(1);
                    resolution(&mut out, r);
                }
                Event::Acknowledged(a) => {
                    out.push(2);
                    resolution(&mut out, a.resolution);
                    out.extend(a.at.to_le_bytes());
                    pos(&mut out, a.place);
                    out.push(a.evidence as u8);
                    out.push(a.regard_change as u8);
                }
                Event::Unreported { resolution: r, at } => {
                    out.push(3);
                    resolution(&mut out, r);
                    out.extend(at.to_le_bytes());
                }
            }
        }
        out.extend((self.observed.len() as u32).to_le_bytes());
        for e in &self.observed {
            out.extend(e.id.to_le_bytes());
            out.extend(e.grower.get().to_le_bytes());
            out.extend(e.carrier.get().to_le_bytes());
            out.push(e.result as u8);
            out.extend(e.at.to_le_bytes());
            pos(&mut out, e.place);
            out.push(e.evidence as u8);
        }
        out
    }
}
impl World {
    pub fn obligations(&self) -> Option<&Obligations> {
        self.obligations.as_ref()
    }
    pub fn witnessed_obligations(&self) -> impl DoubleEndedIterator<Item = &Witnessed> {
        self.obligations()
            .into_iter()
            .flat_map(|o| o.observed.iter())
    }
    pub(super) fn can_offer_commitment(&self, grower: ActorId, carrier: ActorId) -> bool {
        self.obligations().is_none_or(|o| {
            !o.pending.contains_key(&carrier)
                && self.tick >= o.reliability(grower, carrier).retry_at
        })
    }
    pub(super) fn open_obligation(&mut self, team: Team) {
        if let Some(o) = self.obligations.as_mut() {
            o.open(team);
        }
    }
    pub(super) fn close_obligation(
        &mut self,
        team: Team,
        outcome: cooperation::Outcome,
        place: Position,
    ) {
        if let Some(o) = self.obligations.as_mut() {
            let resolution = Resolution {
                team,
                at: self.tick,
                place,
                result: ResultKind::from_outcome(outcome),
            };
            o.resolve(resolution);
            self.open_message(resolution);
        }
    }
    /// A current commitment promotes its existing delivery goal above
    /// optional low-priority work, never above health, fatigue, or duties.
    pub(super) fn obligation_delivery_goal(&self, n: &Npc) -> Option<Goal> {
        self.obligations()?;
        let team = self.cooperation()?.active().get(&n.id())?;
        if !n.alive()
            || n.hp != n.definition.hp
            || self.injuries()?.people[&n.id()].injured()
            || n.herbs != team.shipment
            || self.tick >= team.deadline
            || self.companion(n.id()).is_some()
            || n.fatigue >= n.definition.rest_threshold
            || self.sect_duty_active(n.id())
        {
            return None;
        }
        Some(match team.route {
            Route::Qinghe => Goal::DeliverFreight,
            Route::Willowford => Goal::DeliverFrontierFreight,
        })
    }
    /// A carrier reports its own committed outcome at physical contact, or
    /// the grower reads a matching public manifest at the destination.
    /// The observer does not learn anything merely because time elapsed.
    pub(super) fn advance_obligations(&mut self) {
        let Some(o) = self.obligations() else {
            return;
        };
        let pending = o.pending.values().copied().collect::<Vec<_>>();
        for r in pending {
            let t = r.team;
            let g = &self.npcs[&t.grower];
            let c = &self.npcs[&t.carrier];
            if !g.alive() || self.tick.saturating_sub(r.at) >= REPORT_LIFETIME {
                self.obligations
                    .as_mut()
                    .unwrap()
                    .forget(t.carrier, self.tick);
                self.finish_message(
                    r,
                    super::communication::Kind::Unreported,
                    t.carrier,
                    r.place,
                    false,
                );
                continue;
            }
            let destination = t.route.destination(self);
            let manifest = r.result == ResultKind::Fulfilled
                && in_reach(g.position, destination)
                && self.can_see(g.position, destination)
                && self
                    .cooperation()
                    .unwrap()
                    .register(t.route)
                    .any(|m| m.id == t.id && m.delivered);
            let conversation = c.alive()
                && in_reach(g.position, c.position)
                && self.can_see(g.position, c.position)
                && !self.social.relationship(t.grower, t.carrier).refuses_aid()
                && !self.social.relationship(t.carrier, t.grower).refuses_aid()
                && !matches!(g.activity.goal, Goal::Retreat | Goal::Retaliate)
                && !matches!(c.activity.goal, Goal::Retreat | Goal::Retaliate);
            let relay = self.relay_delivery(r);
            let evidence = if conversation {
                Evidence::Conversation
            } else if manifest {
                Evidence::Register
            } else if relay.is_some() {
                Evidence::Relayed
            } else {
                // A dead carrier cannot report losses, but an already posted
                // successful manifest can still be read until notice expiry.
                if !c.alive()
                    && r.result != ResultKind::Fulfilled
                    && !self.has_relay(r.team.carrier)
                {
                    self.obligations
                        .as_mut()
                        .unwrap()
                        .forget(t.carrier, self.tick);
                    self.finish_message(
                        r,
                        super::communication::Kind::Unreported,
                        t.carrier,
                        r.place,
                        false,
                    );
                }
                continue;
            };
            let place = g.position;
            // Only a witnessed conversation exposes acknowledgement itself.
            // A grower reading a register silently does not expose private reaction.
            let seen = evidence == Evidence::Conversation
                && self.player.alive()
                && self.player_sees(place)
                && self.player_sees(c.position);
            let regard_change = match r.result {
                ResultKind::Fulfilled => self.social.promise_fulfilled(t.grower, t.carrier),
                ResultKind::Released => self.social.promise_disappointment(t.grower, t.carrier),
                _ => 0,
            };
            let ack = Acknowledgement {
                resolution: r,
                at: self.tick,
                place,
                evidence,
                regard_change,
            };
            let (kind, holder, public) = match evidence {
                Evidence::Conversation => (super::communication::Kind::Direct, t.carrier, seen),
                Evidence::Register => (super::communication::Kind::Register, t.carrier, false),
                Evidence::Relayed => {
                    let holder = relay.expect("validated relay");
                    (
                        super::communication::Kind::Relayed,
                        holder,
                        self.player_sees(place) && self.player_sees(self.npcs[&holder].position),
                    )
                }
            };
            self.obligations.as_mut().unwrap().acknowledge(ack, seen);
            self.finish_message(r, kind, holder, place, public);
        }
    }
}
