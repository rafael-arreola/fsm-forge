//! `Review -> Review`: edit in place (self-loop).

use fsm_forge::Edge;

use crate::machine::{ArticleEvent, ArticleMachine, ArticleStage};

pub struct Edit;

impl Edge<ArticleMachine> for Edit {
    const EVENT: ArticleEvent = ArticleEvent::Edit;
    const FROM: &'static [ArticleStage] = &[ArticleStage::Review];
    const TO: ArticleStage = ArticleStage::Review;
}
