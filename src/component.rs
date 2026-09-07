//! Reusable UI component abstractions for Iced.
//!
//! A [`Component`] combines Elm model state management with declarative view rendering.

use iced::Element;

use crate::model::Model;

/// A self-contained UI component with model state and rendering logic.
///
/// Implements [`Model`] for state transitions while providing `render` and `view` methods
/// to draw the component as an [`Element`].
pub trait Component: Model {
    /// Renders the component into an [`Element`] producing internal messages.
    fn render<'a>(&'a self, context: Self::Context<'a>) -> Element<'a, Self::Message>;

    /// Renders the component into an [`Element`] producing parent messages `M`.
    fn render_into<'a, M>(&'a self, context: Self::Context<'a>) -> Element<'a, M>
    where
        Self::Message: Into<M>,
        M: 'a,
    {
        self.render(context).map(Into::into)
    }

    /// Alias for [`Component::render`].
    fn view<'a>(&'a self, context: Self::Context<'a>) -> Element<'a, Self::Message> {
        self.render(context)
    }

    /// Alias for [`Component::render_into`].
    fn view_into<'a, M>(&'a self, context: Self::Context<'a>) -> Element<'a, M>
    where
        Self::Message: Into<M>,
        M: 'a,
    {
        self.render_into(context)
    }
}
