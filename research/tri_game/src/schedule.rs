//! Experimental village calendar. Tick count, in-game minute, and any future
//! render frame stay independent. Nothing here is installed into the kernel.

pub const TICKS_PER_MINUTE: u64 = 2;
pub const MINUTES_PER_DAY: u16 = 1440;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Farmer,
    Smith,
    Disciple,
    Hauler,
    Villager,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activity {
    Sleep,
    Eat,
    Work,
    Socialize,
    Travel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resident {
    pub id: u32,
    pub role: Role,
    pub site: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Village {
    pub tick: u64,
    pub residents: Vec<Resident>,
}

impl Role {
    pub fn from_id(id: u32) -> Self {
        match (id - 1) % 5 {
            0 => Self::Farmer,
            1 => Self::Smith,
            2 => Self::Disciple,
            3 => Self::Hauler,
            _ => Self::Villager,
        }
    }

    pub const fn workplace(self) -> u8 {
        match self {
            Self::Farmer | Self::Hauler => 2,
            Self::Smith => 3,
            Self::Disciple => 4,
            Self::Villager => 1,
        }
    }
}

pub const fn minute_of_day(tick: u64) -> u16 {
    ((tick / TICKS_PER_MINUTE) % 1440) as u16
}

/// Clock blocks from the selective-adoption spec. 05:00–06:00 has no named
/// block, so the resident remains at the bed and the observation stays Sleep.
pub const fn scheduled(minute: u16) -> Activity {
    match minute {
        360..=419 | 720..=779 => Activity::Eat,
        420..=719 | 780..=1019 => Activity::Work,
        1020..=1259 => Activity::Socialize,
        _ => Activity::Sleep,
    }
}

pub const fn destination(role: Role, minute: u16) -> u8 {
    match scheduled(minute) {
        Activity::Work => role.workplace(),
        Activity::Socialize => 5,
        Activity::Sleep | Activity::Eat | Activity::Travel => 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BytesError;

impl Village {
    pub fn settlement(count: u32) -> Self {
        let residents = (1..=count)
            .map(|id| Resident {
                id,
                role: Role::from_id(id),
                site: 1,
            })
            .collect();
        Self { tick: 0, residents }
    }

    pub fn advance(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick = self.tick.saturating_add(1);
            if self.tick.is_multiple_of(TICKS_PER_MINUTE) {
                let minute = minute_of_day(self.tick);
                for resident in &mut self.residents {
                    let goal = destination(resident.role, minute);
                    if resident.site != goal {
                        resident.site = if resident.site < goal {
                            resident.site + 1
                        } else {
                            resident.site - 1
                        };
                    }
                }
            }
        }
    }

    pub fn observations(&self) -> Vec<Activity> {
        let minute = minute_of_day(self.tick);
        self.residents
            .iter()
            .map(|resident| {
                let goal = destination(resident.role, minute);
                if resident.site == goal {
                    scheduled(minute)
                } else {
                    Activity::Travel
                }
            })
            .collect()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.residents.len() * 6);
        bytes.extend(self.tick.to_le_bytes());
        for resident in &self.residents {
            bytes.extend(resident.id.to_le_bytes());
            bytes.push(resident.role as u8);
            bytes.push(resident.site);
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BytesError> {
        if bytes.len() < 8 || !(bytes.len() - 8).is_multiple_of(6) {
            return Err(BytesError);
        }
        let tick = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let (chunks, _) = bytes[8..].as_chunks::<6>();
        let residents = chunks
            .iter()
            .map(|chunk| {
                let id = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
                let role = Role::from_id(id);
                if chunk[4] != role as u8 {
                    return None;
                }
                Some(Resident {
                    id,
                    role,
                    site: chunk[5],
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(BytesError)?;
        Ok(Self { tick, residents })
    }
}

pub fn transition_count(count: u32) -> usize {
    let mut village = Village::settlement(count);
    let mut previous = village.observations();
    let mut changes = 0usize;
    for _ in 0..MINUTES_PER_DAY {
        village.advance(TICKS_PER_MINUTE);
        let next = village.observations();
        changes += previous
            .iter()
            .zip(&next)
            .filter(|(before, after)| before != after)
            .count();
        previous = next;
    }
    changes
}
