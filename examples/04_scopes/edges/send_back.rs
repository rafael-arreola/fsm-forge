//! `Review -> Draft`: send the article back for rework.

use fsm_forge::Edge;

use crate::machine::{ArticleEvent, ArticleMachine, ArticleStage};

pub struct SendBack;

impl Edge<ArticleMachine> for SendBack {
    const EVENT: ArticleEvent = ArticleEvent::SendBack;
    const FROM: &'static [ArticleStage] = &[ArticleStage::Review];
    const TO: ArticleStage = ArticleStage::Draft;
}
