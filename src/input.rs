//! An abstract input state object that gets fed user
//! events and updates itself based on a set of key
//! bindings.
//! The idea is threefold:
//!
//! * Have a layer of abstract key bindings rather than
//! looking at concrete event types
//! * Use this to be able to abstract away differences
//! between keyboards, joysticks and game controllers
//! (rather based on Unity3D),
//! * Do some tweening of input axes and stuff just for
//! fun maybe.
//!
//! Right now ggez doesn't handle joysticks or controllers
//! anyway, so.

use std::hash::Hash;
use std::collections::HashMap;
use ggez::event::*;


// Okay, but how does it actually work?
// Basically we have to bind input events to buttons and axes.
// Input events can be keys, mouse buttons/motion, or eventually
// joystick/controller inputs.  Mouse delta can be mapped to axes too.
//
// https://docs.unity3d.com/Manual/ConventionalGameInput.html has useful
// descriptions of the exact behavior of axes.
//
// So to think about this more clearly, here are the default bindings:
//
// W, ↑: +Y axis
// A, ←: -X axis
// S, ↓: -Y axis
// D, →: +X axis
// Enter, z, LMB: Button 1
// Shift, x, MMB: Button 2
// Ctrl,  c, RMB: Button 3
//
// Easy way?  Hash map of event -> axis/button bindings.

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
enum InputEvent {
    KeyEvent(Keycode), // MouseButtonEvent,
}

#[derive(Debug, Copy, Clone)]
enum InputEffect<Axes, Buttons>
    where Axes: Eq + Hash + Clone,
          Buttons: Eq + Hash + Clone
{
    Axis(Axes, bool),
    Button(Buttons),
}

#[derive(Debug)]
struct AxisStatus {
    // Where the axis currently is, in [-1, 1]
    position: f64,
    // Where the axis is moving towards.  Possible
    // values are -1, 0, +1
    // (or a continuous range for analog devices I guess)
    direction: f64,
    // Speed in units per second that the axis
    // moves towards the target value.
    acceleration: f64,
    // Speed in units per second that the axis will
    // fall back toward 0 if the input stops.
    gravity: f64,
}

impl Default for AxisStatus {
    fn default() -> Self {
        AxisStatus {
            position: 0.0,
            direction: 0.0,
            acceleration: 4.0,
            gravity: 3.0,
        }
    }
}

#[derive(Debug)]
pub struct InputManager<Axes, Buttons>
    where Axes: Hash + Eq + Clone,
          Buttons: Hash + Eq + Clone
{
    // Once EnumSet is stable it should be used for these
    // instead of BTreeMap. ♥?
    // Binding of keys to input values.
    bindings: HashMap<InputEvent, InputEffect<Axes, Buttons>>,
    // Input state for axes
    axes: HashMap<Axes, AxisStatus>,
    // Input states for buttons
    buttons: HashMap<Buttons, bool>,
}

impl<Axes, Buttons> InputManager<Axes, Buttons>
    where Axes: Eq + Hash + Clone,
          Buttons: Eq + Hash + Clone
{
    pub fn new() -> Self {
        InputManager {
            bindings: HashMap::new(),
            axes: HashMap::new(),
            buttons: HashMap::new(),
        }
    }

    /// Adds a key binding connecting the given keycode to the given
    /// logical axis.
    pub fn bind_key_to_axis(mut self, keycode: Keycode, axis: Axes, positive: bool) -> Self {

        self.bindings.insert(InputEvent::KeyEvent(keycode),
                             InputEffect::Axis(axis.clone(), positive));
        self.axes.insert(axis, AxisStatus::default());
        self
    }

    /// Adds a key binding connecting the given keycode to the given
    /// logical button.
    pub fn bind_key_to_button(mut self, keycode: Keycode, button: Buttons) -> Self {
        self.bindings.insert(InputEvent::KeyEvent(keycode),
                             InputEffect::Button(button.clone()));
        self.buttons.insert(button, false);
        self
    }

    /// Updates the logical input state based on the actual
    /// physical input state.  Should be called in your update()
    /// handler.
    /// So, it will do things like move the axes and so on.
    pub fn update(&mut self, dt: f64) {
        for (_axis, axis_status) in self.axes.iter_mut() {
            if axis_status.direction != 0.0 {
                // Accelerate the axis towards the
                // input'ed direction.
                let abs_dx = f64::min(axis_status.acceleration * dt,
                                      1.0 - f64::abs(axis_status.position));
                let dx = if axis_status.direction > 0.0 {
                    abs_dx
                } else {
                    -abs_dx
                };
                axis_status.position += dx;
            } else {
                // Gravitate back towards 0.
                let abs_dx = f64::min(axis_status.gravity * dt, f64::abs(axis_status.position));
                let dx = if axis_status.position > 0.0 {
                    -abs_dx
                } else {
                    abs_dx
                };
                axis_status.position += dx;
            }
        }
    }

    /// This method should get called by your key_down_event handler.
    pub fn update_keydown(&mut self, keycode: Option<Keycode>) {
        if let Some(keycode) = keycode {
            let effect = {
                if let Some(e) = self.bindings.get(&InputEvent::KeyEvent(keycode)) {
                    e.clone()
                } else {
                    return;
                }
            };
            self.update_effect(effect, true);
        }
    }

    /// This method should get called by your key_up_event handler.
    pub fn update_keyup(&mut self, keycode: Option<Keycode>) {
        if let Some(keycode) = keycode {
            let effect = {
                if let Some(e) = self.bindings.get(&InputEvent::KeyEvent(keycode)) {
                    e.clone()
                } else {
                    return;
                }
            };
            self.update_effect(effect, false);
        }
    }

    /// Takes an InputEffect and actually applies it.
    fn update_effect(&mut self, effect: InputEffect<Axes, Buttons>, started: bool) {
        match effect {
            InputEffect::Axis(axis, direction) => {
                let f = || AxisStatus::default();
                let axis_status = self.axes.entry(axis).or_insert_with(f);
                if started {
                    let direction_float = if direction { 1.0 } else { -1.0 };
                    axis_status.direction = direction_float;
                } else {
                    axis_status.direction = 0.0;
                }
            }
            InputEffect::Button(button) => {
                let button_pressed = self.buttons.entry(button).or_insert(started);
                *button_pressed = started;

            }
        }
    }

    pub fn get_axis(&mut self, axis: Axes) -> f64 {
        let f = || AxisStatus::default();
        let axis_status = self.axes.entry(axis).or_insert_with(f);
        axis_status.position
    }

    pub fn get_axis_raw(&mut self, axis: Axes) -> f64 {
        let f = || AxisStatus::default();
        let axis_status = self.axes.entry(axis).or_insert_with(f);
        axis_status.direction
    }

    pub fn get_button(&self, axis: Buttons) -> bool {
        if let Some(pressed) = self.buttons.get(&axis) {
            *pressed
        } else {
            false
        }
    }

    pub fn get_button_down(&self, axis: Buttons) -> bool {
        self.get_button(axis)
    }

    pub fn get_button_up(&self, axis: Buttons) -> bool {
        !self.get_button(axis)
    }

    pub fn mouse_position() {}

    pub fn mouse_scroll_delta() {}

    pub fn get_mouse_button() {}

    pub fn get_mouse_button_down() {}

    pub fn get_mouse_button_up() {}

    pub fn reset_input_axes(&mut self) {
        for (_axis, axis_status) in self.axes.iter_mut() {
            axis_status.position = 0.0;
            axis_status.direction = 0.0;
        }
    }
}


#[cfg(test)]
mod tests {
    use ggez::event::*;
    use super::*;

    #[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
    enum Buttons {
        A,
        B,
        Select,
        Start,
    }

    #[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
    enum Axes {
        Horz,
        Vert,
    }
    #[test]
    fn test_input_events() {
        let mut im = InputManager::<Axes, Buttons>::new()
            .bind_key_to_button(Keycode::Z, Buttons::A)
            .bind_key_to_button(Keycode::X, Buttons::B)
            .bind_key_to_button(Keycode::Return, Buttons::Start)
            .bind_key_to_button(Keycode::RShift, Buttons::Select)
            .bind_key_to_axis(Keycode::Up, Axes::Vert, true)
            .bind_key_to_axis(Keycode::Down, Axes::Vert, false)
            .bind_key_to_axis(Keycode::Left, Axes::Horz, false)
            .bind_key_to_axis(Keycode::Right, Axes::Horz, true);

        im.update_keydown(Some(Keycode::Z));
        assert!(im.get_button(Buttons::A));
        assert!(im.get_button_down(Buttons::A));
        im.update_keyup(Some(Keycode::Z));
        assert!(!im.get_button(Buttons::A));
        assert!(im.get_button_up(Buttons::A));

        // Push the 'up' button, watch the axis
        // increase to 1.0 but not beyond
        im.update_keydown(Some(Keycode::Up));
        assert!(im.get_axis_raw(Axes::Vert) > 0.0);
        while im.get_axis(Axes::Vert) < 0.99 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) >= 0.0);
            assert!(im.get_axis(Axes::Vert) <= 1.0);
        }
        // Release it, watch it wind down
        im.update_keyup(Some(Keycode::Up));
        while im.get_axis(Axes::Vert) > 0.01 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) >= 0.0)
        }

        // Do the same with the 'down' button.
        im.update_keydown(Some(Keycode::Down));
        while im.get_axis(Axes::Vert) > -0.99 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) <= 0.0);
            assert!(im.get_axis(Axes::Vert) >= -1.0);
        }
    }
}
