//! `Review -> Published`: approve and publish.

use fsm_forge::Edge;

use crate::machine::{ArticleEvent, ArticleMachine, ArticleStage};

pub struct Approve;

impl Edge<ArticleMachine> for Approve {
    const EVENT: ArticleEvent = ArticleEvent::Approve;
    const FROM: &'static [ArticleStage] = &[ArticleStage::Review];
    const TO: ArticleStage = ArticleStage::Published;
}
