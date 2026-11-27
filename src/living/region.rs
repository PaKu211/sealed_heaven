//! The two settlements share one turn, herb ledger and actor namespace.

use super::navigation::in_reach;
use super::{ActorId, Outcome, PLAYER, PLAYER_HP, Position, Rejection, Role, World};

use super::content::{
    COIN_BUDGET, GARDEN_CAPACITY, GARDEN_REGROW_EVERY, MARKET_RESERVE, MARKET_TARGET, NORMAL_BUY,
    NORMAL_SELL, OPENING_ORDER, ORDER_INTERVAL, ORDER_TRAVEL, REGION_SITES, REMEDY_HERBS,
    REMEDY_HP, REMEDY_OUTPUT, SCARCE_BUY, SCARCE_SELL, STOCK_BASE, STOCK_VARIANTS, starting_coins,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Background {
    Wanderer,
    Herbalist,
    Peddler,
}

impl Background {
    fn starting_coins(self) -> u32 {
        starting_coins(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionSites {
    pub market: Position,
    pub garden: Position,
    pub hall: Position,
}

impl RegionSites {
    pub(super) fn positions(self) -> [Position; 3] {
        [self.market, self.garden, self.hall]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarketQuote {
    pub buy: u32,
    pub sell: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub seed: u32,
    pub background: Option<Background>,
    pub coins: u32,
    pub market_coins: u32,
    pub market_herbs: u32,
    pub garden_herbs: u32,
    pub garden_regrown: u32,
    pub shipped_to_qinghe: u32,
    pub hall_offerings: u32,
    pub enrolled: bool,
    pub remedies: u32,
    pub remedies_used: u32,
    pub(crate) order_remaining: u32,
    pub(crate) order_in_transit: Option<(u64, u32)>,
}

impl Region {
    pub(super) fn new(seed: u32) -> Self {
        Self {
            seed,
            background: None,
            coins: 0,
            market_coins: COIN_BUDGET,
            market_herbs: Self::initial_stock(seed),
            garden_herbs: GARDEN_CAPACITY,
            garden_regrown: 0,
            shipped_to_qinghe: 0,
            hall_offerings: 0,
            enrolled: false,
            remedies: 0,
            remedies_used: 0,
            // Qinghe's opening order was sent before the campaign begins.
            order_remaining: OPENING_ORDER,
            order_in_transit: None,
        }
    }

    fn initial_stock(seed: u32) -> u32 {
        // The next seed bits govern stock independently of which road is open.
        STOCK_BASE + (seed >> 1) % STOCK_VARIANTS
    }

    pub fn sites() -> RegionSites {
        REGION_SITES
    }

    pub fn quote(self) -> MarketQuote {
        let scarce = self.market_herbs <= MARKET_RESERVE;
        MarketQuote {
            buy: if scarce { SCARCE_BUY } else { NORMAL_BUY },
            sell: if scarce { SCARCE_SELL } else { NORMAL_SELL },
        }
    }
}

impl World {
    /// The market's quoted price includes the local consequence of sect unrest.
    /// Older campaign revisions retain their original market quotations.
    pub fn market_quote(&self) -> Option<MarketQuote> {
        let mut quote = self.region()?.quote();
        if self
            .rivalry()
            .is_some_and(|rival| rival.contested(self.tick))
        {
            quote.buy += 1;
        }
        Some(quote)
    }

    pub(super) fn select_default_background(&mut self) {
        if self.region.is_some() {
            self.choose_background(Background::Wanderer)
                .expect("first turn");
        }
    }

    pub(super) fn choose_background(
        &mut self,
        background: Background,
    ) -> Result<Outcome, Rejection> {
        let region = self.region.as_mut().expect("regional rules");
        if self.tick != 1 || region.background.is_some() {
            return Err(Rejection::BackgroundLocked);
        }
        let endowment = background.starting_coins();
        region.background = Some(background);
        region.market_coins -= endowment;
        region.coins += endowment;
        Ok(Outcome::BackgroundChosen(background))
    }

    pub(super) fn gather(&mut self) -> Result<Outcome, Rejection> {
        if self.frontier().is_some() && in_reach(self.player.position, super::expansion::GROVE) {
            return self.gather_frontier();
        }
        let (source, limit) = if let Some(region) = &mut self.region {
            let source = if in_reach(self.player.position, Region::sites().garden) {
                &mut region.garden_herbs
            } else if in_reach(self.player.position, self.scenario.patch) {
                &mut self.supply.source
            } else {
                return Err(Rejection::OutOfReach);
            };
            (
                source,
                if region.background == Some(Background::Herbalist) {
                    2
                } else {
                    1
                },
            )
        } else {
            self.require_reach(self.scenario.patch)?;
            (&mut self.supply.source, 1)
        };
        let amount = (*source).min(limit);
        if amount == 0 {
            return Err(Rejection::Empty);
        }
        *source -= amount;
        self.player.herbs += amount;
        Ok(if amount == 2 {
            Outcome::GatheredPair
        } else {
            Outcome::Gathered
        })
    }

    pub(super) fn advance_region(&mut self) {
        let garden_cadence = self.generated_region().map_or(
            GARDEN_REGROW_EVERY,
            super::worldgen::GeneratedRegion::garden_regrow_every,
        );
        let shortage = if self.region.is_some() && self.tick.is_multiple_of(ORDER_INTERVAL) {
            Some(self.west_shortage())
        } else {
            None
        };
        if let Some(region) = &mut self.region {
            if self.tick.is_multiple_of(garden_cadence) && region.garden_herbs < GARDEN_CAPACITY {
                region.garden_herbs += 1;
                region.garden_regrown += 1;
            }
            if let Some((due, amount)) = region.order_in_transit
                && self.tick >= due
            {
                region.order_remaining = amount;
                region.order_in_transit = None;
            }
            if let Some(amount) = shortage {
                region.order_in_transit = Some((self.tick + ORDER_TRAVEL, amount));
            }
        }
    }

    pub(super) fn trade_herb(&mut self, buying: bool) -> Result<Outcome, Rejection> {
        if self.frontier().is_some() && in_reach(self.player.position, super::expansion::DEPOT) {
            return self.trade_frontier(buying);
        }
        self.require_reach(Region::sites().market)?;
        if self.is_night() {
            return Err(Rejection::MarketClosed);
        }
        let quote = self.market_quote().expect("regional rules");
        let region = self.region.as_mut().expect("regional rules");
        let coins = if buying {
            if region.market_herbs == 0 {
                return Err(Rejection::Empty);
            }
            if region.coins < quote.buy {
                return Err(Rejection::NotEnoughCoins);
            }
            region.market_herbs -= 1;
            self.player.herbs += 1;
            region.coins -= quote.buy;
            region.market_coins += quote.buy;
            quote.buy
        } else {
            if self.player.herbs == 0 {
                return Err(Rejection::NoCargo);
            }
            if region.market_coins < quote.sell {
                return Err(Rejection::NotEnoughCoins);
            }
            self.player.herbs -= 1;
            region.market_herbs += 1;
            region.market_coins -= quote.sell;
            region.coins += quote.sell;
            quote.sell
        };
        self.record_regional_event(
            super::regional_news::Site::WillowfordMarket,
            if buying {
                super::regional_news::EventKind::Bought
            } else {
                super::regional_news::EventKind::Sold
            },
            Some(PLAYER),
            1,
        );
        if !buying {
            self.request_contribution(super::requests::Site::Market);
        }
        Ok(Outcome::Traded {
            bought: buying,
            coins,
        })
    }

    pub(super) fn hall_mentor(&self) -> Result<ActorId, Rejection> {
        let hall = Region::sites().hall;
        self.require_reach(hall)?;
        if self
            .politics()
            .is_some_and(|politics| politics.allegiance == super::Allegiance::Stonebank)
        {
            return Err(Rejection::OathConflict);
        }
        if self.is_night() {
            return Err(Rejection::HallClosed);
        }
        if self
            .rivalry()
            .is_some_and(|rival| rival.contested(self.tick))
        {
            return Err(Rejection::SchoolContested);
        }
        let mentor = self
            .npcs
            .values()
            .find(|npc| self.is_hall_teacher(npc) && npc.alive() && npc.position == hall)
            .ok_or(Rejection::HallClosed)?;
        if self.social.relationship(mentor.id(), PLAYER).refuses_aid() {
            return Err(Rejection::Refused);
        }
        Ok(mentor.id())
    }

    pub(super) fn join_hall(&mut self) -> Result<Outcome, Rejection> {
        let mentor = self.hall_mentor()?;
        let region = self.region.as_mut().expect("regional rules");
        if region.enrolled {
            return Err(Rejection::AlreadyEnrolled);
        }
        if self.breath == 0 {
            return Err(Rejection::PracticeRequired);
        }
        if self.player.herbs == 0 {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= 1;
        region.hall_offerings += 1;
        region.enrolled = true;
        self.social.meet(mentor, PLAYER);
        self.enroll_hierarchy_player();
        Ok(Outcome::Enrolled)
    }

    pub(super) fn brew_remedy(&mut self) -> Result<Outcome, Rejection> {
        self.hall_mentor()?;
        let region = self.region.as_mut().expect("regional rules");
        if !region.enrolled {
            return Err(Rejection::MembershipRequired);
        }
        if self.player.herbs < REMEDY_HERBS {
            return Err(Rejection::NoCargo);
        }
        self.player.herbs -= REMEDY_HERBS;
        region.remedies += REMEDY_OUTPUT;
        Ok(Outcome::Brewed)
    }

    pub(super) fn use_remedy(&mut self) -> Result<Outcome, Rejection> {
        let region = self.region.as_mut().expect("regional rules");
        if region.remedies == 0 {
            return Err(Rejection::NoRemedy);
        }
        let healed = (PLAYER_HP - self.player.hp).min(REMEDY_HP);
        if healed == 0 {
            return Err(Rejection::Uninjured);
        }
        region.remedies -= 1;
        region.remedies_used += 1;
        self.player.hp += healed;
        Ok(Outcome::UsedRemedy { healed })
    }

    pub(super) fn needs_east_harvest(&self) -> bool {
        self.region
            .as_ref()
            .is_some_and(|region| region.market_herbs < MARKET_TARGET && region.garden_herbs > 0)
    }

    pub(super) fn needs_courier_supply(&self) -> bool {
        self.region.as_ref().is_some_and(|region| {
            region.market_herbs > MARKET_RESERVE && region.order_remaining > 0
        })
    }

    pub(super) fn west_shortage(&self) -> u32 {
        let in_transit: u32 = self
            .npcs
            .values()
            .filter(|npc| {
                npc.alive() && matches!(npc.definition.role, Role::Courier | Role::Supplier)
            })
            .map(|npc| npc.herbs)
            .sum();
        self.scenario
            .shrine_target
            .saturating_sub(self.supply.shrine + in_transit)
    }

    pub(super) fn load_regional_cargo(&mut self, id: ActorId, east_grower: bool) {
        let region = self.region.as_mut().expect("regional worker");
        let amount = if east_grower {
            let missing = MARKET_TARGET.saturating_sub(region.market_herbs);
            let amount = region.garden_herbs.min(missing).min(2);
            region.garden_herbs -= amount;
            amount
        } else {
            let amount = region
                .market_herbs
                .saturating_sub(MARKET_RESERVE)
                .min(region.order_remaining)
                .min(2);
            region.market_herbs -= amount;
            region.order_remaining -= amount;
            amount
        };
        self.npcs.get_mut(&id).expect("regional worker").herbs += amount;
        if !east_grower && amount > 0 {
            self.record_regional_event(
                super::regional_news::Site::WillowfordMarket,
                super::regional_news::EventKind::FreightLoaded,
                Some(id),
                amount,
            );
        }
    }

    pub(super) fn unload_regional_cargo(&mut self, id: ActorId, east_grower: bool) {
        let region = self.region.as_mut().expect("regional worker");
        let amount = std::mem::take(&mut self.npcs.get_mut(&id).expect("regional worker").herbs);
        if east_grower {
            region.market_herbs += amount;
        } else {
            self.supply.shrine += amount;
            self.supply.npc_deliveries += amount;
            region.shipped_to_qinghe += amount;
        }

        if amount > 0 {
            use super::regional_news::{EventKind, Site};
            let (site, kind) = if east_grower {
                (Site::WillowfordMarket, EventKind::MarketStocked)
            } else {
                (Site::QingheShrine, EventKind::FreightArrived)
            };
            self.record_regional_event(site, kind, Some(id), amount);
        }
    }
}
