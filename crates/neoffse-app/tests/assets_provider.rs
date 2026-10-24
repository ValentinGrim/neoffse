//! The provider trait, the chain and the default provider.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::sync::Arc;

use neoffse::core::Game;
use neoffse_app::assets::{AssetProvider, OriginalSprites, ProviderChain, default_provider};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// A provider that answers every query with a square image of a set side, or
/// with nothing.
struct Fake(Option<u32>);

impl AssetProvider for Fake {
    fn portrait(&self, _game: Game, _job: u32) -> Option<Image> {
        self.0.map(|side| Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::new(side, side)))
    }
}

fn chain(sides: &[Option<u32>]) -> ProviderChain {
    ProviderChain::new(sides.iter().map(|side| Arc::new(Fake(*side)) as Arc<dyn AssetProvider>).collect())
}

fn side_of(image: &Image) -> (u32, u32) {
    let size = image.size();
    (size.width, size.height)
}

fn needs_send_and_sync<T: Send + Sync + ?Sized>(_: &T) {}

#[test]
fn the_chain_returns_the_first_some() {
    let image = chain(&[None, Some(3), Some(4)]).portrait(Game::Ff1Pr, 1).unwrap();
    assert_eq!(side_of(&image), (3, 3));
}

#[test]
fn an_empty_chain_has_no_portrait() {
    assert!(chain(&[]).portrait(Game::Ff1Pr, 1).is_none());
}

#[test]
fn a_chain_skips_a_provider_with_none() {
    let image = chain(&[None, None, Some(7)]).portrait(Game::Ff1Pr, 1).unwrap();
    assert_eq!(side_of(&image), (7, 7));
    assert!(chain(&[None, None]).portrait(Game::Ff1Pr, 1).is_none());
}

#[test]
fn a_chain_of_one_original_sprites_serves_ff1() {
    let provider = default_provider();
    assert!(provider.portrait(Game::Ff1Pr, 1).is_some());
    assert!(provider.portrait(Game::Ff1Pr, 0).is_none());
}

#[test]
fn providers_are_send_and_sync() {
    let shared: Arc<dyn AssetProvider> = default_provider();
    needs_send_and_sync(&shared);
    needs_send_and_sync(&chain(&[]));
    needs_send_and_sync(&OriginalSprites);
    needs_send_and_sync(shared.as_ref());
}

#[test]
fn the_chain_debug_prints_a_count() {
    assert_eq!(format!("{:?}", chain(&[None, Some(1)])), "ProviderChain { providers: 2 }");
    assert_eq!(format!("{:?}", chain(&[])), "ProviderChain { providers: 0 }");
}

#[test]
fn a_chain_is_a_provider_of_a_chain() {
    let inner: Arc<dyn AssetProvider> = Arc::new(chain(&[None, Some(9)]));
    let outer = ProviderChain::new(vec![Arc::new(Fake(None)), inner]);
    assert_eq!(side_of(&outer.portrait(Game::Ff1Pr, 5).unwrap()), (9, 9));
}
