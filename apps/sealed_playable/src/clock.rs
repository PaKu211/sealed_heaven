//! Presentation calendar. It is a pure function of the canonical tick.
//! It does not advance time and it does not describe what an NPC is doing.

pub const TICKS_PER_MINUTE: u64 = 2;
pub const MINUTES_PER_DAY: u64 = 1440;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedBlock {
    Sleep,
    Eat,
    Work,
    Socialize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Daylight {
    Day,
    Night,
}

pub const fn minute_of_day(tick: u64) -> u16 {
    ((tick / TICKS_PER_MINUTE) % MINUTES_PER_DAY) as u16
}

pub const fn day_number(tick: u64) -> u64 {
    (tick / TICKS_PER_MINUTE) / MINUTES_PER_DAY + 1
}

pub const fn daylight(minute: u16) -> Daylight {
    if minute >= 360 && minute < 1080 {
        Daylight::Day
    } else {
        Daylight::Night
    }
}

pub const fn is_night(minute: u16) -> bool {
    matches!(daylight(minute), Daylight::Night)
}

/// A readable label for the clock. This is not a kernel goal.
pub const fn planned_block(minute: u16) -> PlannedBlock {
    match minute {
        360..=419 | 720..=779 => PlannedBlock::Eat,
        420..=719 | 780..=1019 => PlannedBlock::Work,
        1020..=1079 => PlannedBlock::Socialize,
        _ => PlannedBlock::Sleep,
    }
}

pub const fn clock_hhmm(minute: u16) -> (u16, u16) {
    (minute / 60, minute % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_matches_the_milestone_contract() {
        assert_eq!(minute_of_day(0), 0);
        assert_eq!(minute_of_day(1), 0);
        assert_eq!(minute_of_day(2), 1);
        assert_eq!(minute_of_day(2 * 1440), 0);
        assert_eq!(minute_of_day(2 * 1440 + 3), 1);
        assert_eq!(day_number(0), 1);
        assert_eq!(day_number(2 * 1440), 2);
        assert_eq!(clock_hhmm(0), (0, 0));
        assert_eq!(clock_hhmm(360), (6, 0));
        assert_eq!(clock_hhmm(1080), (18, 0));
    }

    #[test]
    fn daylight_and_schedule_are_pure() {
        assert_eq!(daylight(0), Daylight::Night);
        assert_eq!(daylight(359), Daylight::Night);
        assert_eq!(daylight(360), Daylight::Day);
        assert_eq!(daylight(1079), Daylight::Day);
        assert_eq!(daylight(1080), Daylight::Night);
        assert!(is_night(1300));
        assert_eq!(planned_block(0), PlannedBlock::Sleep);
        assert_eq!(planned_block(360), PlannedBlock::Eat);
        assert_eq!(planned_block(420), PlannedBlock::Work);
        assert_eq!(planned_block(1020), PlannedBlock::Socialize);
        assert_eq!(planned_block(1080), PlannedBlock::Sleep);
    }
}
