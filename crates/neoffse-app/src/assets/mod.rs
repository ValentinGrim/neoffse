//! Images for the screens, behind one trait.
//!
//! An [`AssetProvider`] answers "which image shows this character" by game
//! and job id and hides where the image comes from. [`OriginalSprites`] holds
//! the six sprites drawn for the project. A later provider that reads a
//! folder of extracted images joins a [`ProviderChain`] without any change to
//! the screens (`docs/architecture.md#visual-assets`). No library crate
//! depends on the trait: it belongs to the app layer.
//!
//! The job-to-class pairing of FF I is in [`class_of`].

mod sprites;

use std::fmt;
use std::sync::Arc;

use neoffse::core::Game;

pub use sprites::{OriginalSprites, PortraitClass, class_of};

/// A source of images for the screens. The portrait of a character is picked
/// by game and job id.
pub trait AssetProvider: Send + Sync {
    /// The portrait of a character of `game` whose job id is `job`, `None`
    /// when this provider holds none.
    ///
    /// Call it on the UI thread: [`slint::Image`] is not `Send`.
    fn portrait(&self, game: Game, job: u32) -> Option<slint::Image>;
}

/// Providers in order of preference: the first `Some` wins.
#[derive(Clone)]
pub struct ProviderChain {
    providers: Vec<Arc<dyn AssetProvider>>,
}

impl ProviderChain {
    /// A chain that asks `providers` in the given order. An empty chain
    /// holds no image.
    #[must_use]
    pub fn new(providers: Vec<Arc<dyn AssetProvider>>) -> Self {
        Self { providers }
    }
}

impl AssetProvider for ProviderChain {
    fn portrait(&self, game: Game, job: u32) -> Option<slint::Image> {
        self.providers.iter().find_map(|provider| provider.portrait(game, job))
    }
}

impl fmt::Debug for ProviderChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderChain").field("providers", &self.providers.len()).finish()
    }
}

/// The chain the app uses: today the original sprites alone.
#[must_use]
pub fn default_provider() -> Arc<dyn AssetProvider> {
    Arc::new(ProviderChain::new(vec![Arc::new(OriginalSprites)]))
}
