#[cfg(test)]
mod tests {
    use domain::Step;
    use sim_runtime::Clock;
    use task_3_2::{Effect, Event, Phase, Visit};

    #[test]
    fn door_open_uses_the_sim_clock() {
        let mut clock = Clock::new();
        let mut visit = Visit::new();
        assert_eq!(clock.now_ms(), 0);
        let outcome = visit.apply(clock.now_ms(), Event::DoorOpen).unwrap();
        assert_eq!(outcome.effect, Some(Effect::GoToPoint));
        assert_eq!(visit.phase(), Phase::GoingToPoint);
        clock.advance(Step::from_ms(25)).unwrap();
        assert_eq!(clock.now_ms(), 25);
        visit.apply(clock.now_ms(), Event::Tick).unwrap();
        assert_eq!(visit.now_ms(), Some(clock.now_ms()));
        assert_eq!(visit.phase(), Phase::GoingToPoint);
    }
}
