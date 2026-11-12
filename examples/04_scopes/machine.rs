//! The article machine: demonstrates a reusable `Rule` struct and all five `Scope`
//! selectors (`always`, `on`, `entering`, `leaving`, `when`).

use fsm_forge::{BuildErrors, Engine, Machine, Rule, Scope, Step};

use crate::edges::{Approve, Edit, SendBack, Submit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArticleStage {
    Draft,
    Review,
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArticleEvent {
    Submit,
    Edit,
    SendBack,
    Approve,
}

#[derive(Debug, Clone)]
pub struct Article {
    pub stage: ArticleStage,
    pub touches: u32,
}

pub struct ArticleMachine;

impl Machine for ArticleMachine {
    type State = ArticleStage;
    type Event = ArticleEvent;
    type Entity = Article;
    type Input = ();
    type Effect = &'static str;
    type Error = &'static str;

    fn state(article: &Article) -> ArticleStage {
        article.stage
    }

    fn set_state(article: &mut Article, stage: ArticleStage) {
        article.stage = stage;
    }
}

/// Reusable rule struct implementing `Rule`: increments `touches` on every accepted step.
pub struct CountTouch;

impl Rule<ArticleMachine> for CountTouch {
    fn apply(&self, _step: &Step<'_, ArticleMachine>, article: &mut Article) {
        article.touches += 1;
    }
}

pub fn article_engine() -> Result<Engine<ArticleMachine>, BuildErrors<ArticleMachine>> {
    Engine::<ArticleMachine>::builder()
        .edge(Submit)
        .edge(Edit)
        .edge(SendBack)
        .edge(Approve)
        // 1. `Scope::always` + reusable `Rule` struct
        .rule(Scope::always(), CountTouch)
        // 2. `Scope::on`: only specific events
        .emit(
            Scope::on(&[ArticleEvent::Submit, ArticleEvent::Approve]),
            "notify_board",
            "notify_editorial_board",
        )
        // 3. `Scope::entering`: enters `Review` from a different state (excludes `Edit` self-loop)
        .emit(
            Scope::entering(&[ArticleStage::Review]),
            "assign_reviewer",
            "assign_reviewer",
        )
        // 4. `Scope::leaving`: leaves `Review` to a different state (excludes `Edit` self-loop)
        .emit(
            Scope::leaving(&[ArticleStage::Review]),
            "close_review",
            "close_review_round",
        )
        // 5. `Scope::when`: arbitrary predicate over `Step` (here, self-loops only)
        .emit(
            Scope::when(|step| step.is_self_loop()),
            "save_edit",
            "save_inline_edit",
        )
        .initial(ArticleStage::Draft)
        .require_events([
            ArticleEvent::Submit,
            ArticleEvent::Edit,
            ArticleEvent::SendBack,
            ArticleEvent::Approve,
        ])
        .require_states([
            ArticleStage::Draft,
            ArticleStage::Review,
            ArticleStage::Published,
        ])
        .build()
}
