use crate::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Reflect)]
pub struct EmptyScreen;
impl Screen for EmptyScreen {
    fn builder(builder: ScreenScopeBuilder<Self>) -> ScreenScopeBuilder<Self> {
        builder
    }
}
