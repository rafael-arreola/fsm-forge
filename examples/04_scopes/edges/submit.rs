//! `Draft -> Review`: submit the article for review.

use fsm_forge::Edge;

use crate::machine::{ArticleEvent, ArticleMachine, ArticleStage};

pub struct Submit;

impl Edge<ArticleMachine> for Submit {
    const EVENT: ArticleEvent = ArticleEvent::Submit;
    const FROM: &'static [ArticleStage] = &[ArticleStage::Draft];
    const TO: ArticleStage = ArticleStage::Review;
}
