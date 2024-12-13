use std::time::Duration;

/// A simple debouncer that holds the last state and manages debounce timing.
pub struct Debouncer<T> {
    delay: Duration,       // The debounce delay duration
    last_state: Option<T>, // The last received state
    elapsed: Duration,     // Time elapsed since the last event
}

impl<T> Debouncer<T> {
    /// Creates a new Debouncer with the specified delay.
    ///
    /// # Arguments
    ///
    /// * `delay` - The debounce duration.
    pub fn new(delay: Duration) -> Self {
        Debouncer {
            delay,
            last_state: None,
            elapsed: Duration::ZERO,
        }
    }

    /// Pushes a new event/state into the debouncer.
    ///
    /// This updates the last state and resets the elapsed time.
    ///
    /// # Arguments
    ///
    /// * `state` - The new state to debounce.
    pub fn push(&mut self, state: T) {
        self.last_state = Some(state);
        self.elapsed = Duration::ZERO;
    }

    /// Advances the internal timer by the given delta.
    ///
    /// # Arguments
    ///
    /// * `delta` - The time to advance.
    pub fn tick(&mut self, delta: f32) {
        self.elapsed += Duration::from_secs_f32(delta);
    }

    /// Retrieves the debounced state if the delay has passed.
    ///
    /// If the debounce delay has been exceeded since the last event,
    /// it returns `Some(state)` and clears the stored state.
    /// Otherwise, it returns `None`.
    pub fn get(&mut self) -> Option<T> {
        if let Some(ref _state) = self.last_state {
            if self.elapsed >= self.delay {
                // Take the state out, leaving None in its place
                return self.last_state.take();
            }
        }
        None
    }

    /// Peeks at the debounced state without consuming it.
    ///
    /// This returns a reference to the stored state if the delay has passed,
    /// or `None` if the delay has not yet been exceeded.
    pub fn peek(&self) -> Option<&T> {
        if self.elapsed >= self.delay {
            self.last_state.as_ref()
        } else {
            None
        }
    }
}
