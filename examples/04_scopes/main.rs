//! Step 4 — Scopes and reusable `Rule` structs: how `always`, `on`, `entering`,
//! `leaving`, and `when` select transitions (and how self-loops behave).
//!
//! Run with `cargo run --example 04_scopes`.

mod edges;
mod machine;

use crate::machine::{Article, ArticleEvent, ArticleStage, article_engine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = article_engine()?;
    let mut article = Article {
        stage: ArticleStage::Draft,
        touches: 0,
    };

    for event in [
        ArticleEvent::Submit,   // Draft -> Review: always + on + entering(Review)
        ArticleEvent::Edit,     // Review -> Review (self-loop): always + when(is_self_loop)
        ArticleEvent::SendBack, // Review -> Draft: always + leaving(Review)
        ArticleEvent::Submit,   // Draft -> Review: always + on + entering(Review)
        ArticleEvent::Approve,  // Review -> Published: always + on + leaving(Review)
    ] {
        let outcome = engine.execute(&article, event, &())?;
        let effects: Vec<_> = outcome.effect_values().copied().collect();
        println!(
            "{event:?}: {:?} -> {:?}\n  rules:   {:?}\n  effects: {effects:?}",
            outcome.transition.from, outcome.transition.to, outcome.transition.rules
        );
        article = outcome.entity;
    }

    println!(
        "\nfinal stage: {:?}, touches: {}",
        article.stage, article.touches
    );
    Ok(())
}
