//! Scopes: where global rules apply, self-loop boundaries, and from-any mode.

use fsm_forge::{Edge, Engine, Machine, Outcome, Rejection, Scope, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ArticleState {
    Draft,
    Review,
    Published,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ArticleEvent {
    Submit,
    Approve,
    Reject,
    Retry,
    Publish,
    Archive,
    ForcePublish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Article {
    state: ArticleState,
    published: bool,
}

/// Article machine: all five scopes, self-loop boundaries, and from-any mode.
///
/// ```mermaid
/// stateDiagram-v2
///     state "Draft" as Draft
///     state "Review" as Review
///     state "Published" as Published
///     state "Archived" as Archived
///     [*] --> Draft
///     Draft --> Review : Submit
///     Review --> Published : Approve
///     Review --> Draft : Reject
///     Review --> Review : Retry
///     Review --> Published : Publish
///     Published --> Archived : Archive
///     note right of Published : ForcePublish
/// ```
struct ArticleMachine;

impl Machine for ArticleMachine {
    type State = ArticleState;
    type Event = ArticleEvent;
    type Entity = Article;
    type Input = ();
    type Effect = &'static str;
    type Error = &'static str;

    fn state(article: &Article) -> ArticleState {
        article.state
    }

    fn set_state(article: &mut Article, state: ArticleState) {
        article.state = state;
    }
}

struct Submit;

impl Edge<ArticleMachine> for Submit {
    const EVENT: ArticleEvent = ArticleEvent::Submit;
    const FROM: &'static [ArticleState] = &[ArticleState::Draft];
    const TO: ArticleState = ArticleState::Review;
}

struct Approve;

impl Edge<ArticleMachine> for Approve {
    const EVENT: ArticleEvent = ArticleEvent::Approve;
    const FROM: &'static [ArticleState] = &[ArticleState::Review];
    const TO: ArticleState = ArticleState::Published;

    fn apply(&self, _step: &Step<'_, ArticleMachine>, article: &mut Article) {
        article.published = true;
    }

    fn verify(
        &self,
        _step: &Step<'_, ArticleMachine>,
        article: &Article,
    ) -> Result<(), &'static str> {
        if article.published {
            Ok(())
        } else {
            Err("not published")
        }
    }

    fn effects(&self, _step: &Step<'_, ArticleMachine>, _article: &Article) -> Vec<&'static str> {
        vec!["fx:approved"]
    }
}

struct Reject;

impl Edge<ArticleMachine> for Reject {
    const EVENT: ArticleEvent = ArticleEvent::Reject;
    const FROM: &'static [ArticleState] = &[ArticleState::Review];
    const TO: ArticleState = ArticleState::Draft;
}

struct Retry;

impl Edge<ArticleMachine> for Retry {
    const EVENT: ArticleEvent = ArticleEvent::Retry;
    const FROM: &'static [ArticleState] = &[ArticleState::Review];
    const TO: ArticleState = ArticleState::Review;
}

struct Publish;

impl Edge<ArticleMachine> for Publish {
    const EVENT: ArticleEvent = ArticleEvent::Publish;
    const FROM: &'static [ArticleState] = &[ArticleState::Review];
    const TO: ArticleState = ArticleState::Published;

    fn apply(&self, _step: &Step<'_, ArticleMachine>, article: &mut Article) {
        article.published = true;
    }
}

struct Archive;

impl Edge<ArticleMachine> for Archive {
    const EVENT: ArticleEvent = ArticleEvent::Archive;
    const FROM: &'static [ArticleState] = &[ArticleState::Published];
    const TO: ArticleState = ArticleState::Archived;
}

struct ForcePublish;

impl Edge<ArticleMachine> for ForcePublish {
    const EVENT: ArticleEvent = ArticleEvent::ForcePublish;
    const FROM: &'static [ArticleState] = &[];
    const TO: ArticleState = ArticleState::Published;

    fn apply(&self, _step: &Step<'_, ArticleMachine>, article: &mut Article) {
        article.published = true;
    }
}

fn article_engine() -> Engine<ArticleMachine> {
    Engine::<ArticleMachine>::builder()
        .edge(Submit)
        .edge(Approve)
        .edge(Reject)
        .edge(Retry)
        .edge(Publish)
        .edge(Archive)
        .edge(ForcePublish)
        .emit(Scope::always(), "always", "fx:always")
        .emit(
            Scope::on(&[ArticleEvent::Publish, ArticleEvent::Archive]),
            "on",
            "fx:on",
        )
        .emit(
            Scope::entering(&[ArticleState::Published]),
            "entering_published",
            "fx:entering_published",
        )
        .emit(
            Scope::entering(&[ArticleState::Review]),
            "entering_review",
            "fx:entering_review",
        )
        .emit(
            Scope::leaving(&[ArticleState::Draft]),
            "leaving_draft",
            "fx:leaving_draft",
        )
        .emit(
            Scope::leaving(&[ArticleState::Review]),
            "leaving_review",
            "fx:leaving_review",
        )
        .emit(Scope::when(|step| step.from_any), "from_any", "fx:from_any")
        .emit(
            Scope::<ArticleMachine>::when(|step| step.entity.published),
            "already_published",
            "fx:already_published",
        )
        .initial(ArticleState::Draft)
        .require_events([
            ArticleEvent::Submit,
            ArticleEvent::Approve,
            ArticleEvent::Reject,
            ArticleEvent::Retry,
            ArticleEvent::Publish,
            ArticleEvent::Archive,
            ArticleEvent::ForcePublish,
        ])
        .require_states([
            ArticleState::Draft,
            ArticleState::Review,
            ArticleState::Published,
            ArticleState::Archived,
        ])
        .build()
        .expect("valid article machine")
}

fn article_in(state: ArticleState, published: bool) -> Article {
    Article { state, published }
}

fn effects(outcome: &Outcome<ArticleMachine>) -> Vec<&'static str> {
    outcome.effect_values().copied().collect()
}

#[test]
fn submit_matches_always_entering_review_and_leaving_draft() {
    let engine = article_engine();
    let article = article_in(ArticleState::Draft, false);

    let outcome = engine
        .execute(&article, ArticleEvent::Submit, &())
        .expect("submit is valid");

    assert_eq!(outcome.transition.to, ArticleState::Review);
    assert_eq!(
        outcome.transition.rules,
        vec!["Submit", "always", "entering_review", "leaving_draft"]
    );
    assert_eq!(
        effects(&outcome),
        vec!["fx:always", "fx:entering_review", "fx:leaving_draft"]
    );
}

#[test]
fn approve_matches_entering_published_and_leaving_review() {
    let engine = article_engine();
    let article = article_in(ArticleState::Review, false);

    let outcome = engine
        .execute(&article, ArticleEvent::Approve, &())
        .expect("approve is valid");

    assert_eq!(outcome.transition.to, ArticleState::Published);
    assert!(outcome.entity.published);
    assert_eq!(
        outcome.transition.rules,
        vec!["Approve", "always", "entering_published", "leaving_review",]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            "fx:approved",
            "fx:always",
            "fx:entering_published",
            "fx:leaving_review",
        ]
    );

    // Scopes run before `apply`, so the original entity is still unpublished.
    assert!(!outcome.transition.rules.contains(&"already_published"));
}

#[test]
fn self_loop_is_excluded_from_entering_and_leaving() {
    let engine = article_engine();
    let article = article_in(ArticleState::Review, false);

    let outcome = engine
        .execute(&article, ArticleEvent::Retry, &())
        .expect("retry is valid");

    assert!(outcome.transition.is_self_loop());
    assert_eq!(outcome.transition.rules, vec!["Retry", "always"]);
    assert_eq!(effects(&outcome), vec!["fx:always"]);
}

#[test]
fn publish_matches_on_entering_published_and_leaving_review() {
    let engine = article_engine();
    let article = article_in(ArticleState::Review, false);

    let outcome = engine
        .execute(&article, ArticleEvent::Publish, &())
        .expect("publish is valid");

    assert_eq!(
        outcome.transition.rules,
        vec![
            "Publish",
            "always",
            "on",
            "entering_published",
            "leaving_review",
        ]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            "fx:always",
            "fx:on",
            "fx:entering_published",
            "fx:leaving_review",
        ]
    );
}

#[test]
fn archive_matches_on_and_already_published() {
    let engine = article_engine();
    let article = article_in(ArticleState::Published, true);

    let outcome = engine
        .execute(&article, ArticleEvent::Archive, &())
        .expect("archive is valid");

    assert_eq!(
        outcome.transition.rules,
        vec!["Archive", "always", "on", "already_published"]
    );
    assert_eq!(
        effects(&outcome),
        vec!["fx:always", "fx:on", "fx:already_published"]
    );
}

#[test]
fn from_any_matches_only_in_from_any_mode() {
    let engine = article_engine();
    let article = article_in(ArticleState::Draft, false);

    let rejection = engine
        .execute(&article, ArticleEvent::ForcePublish, &())
        .expect_err("edge without origin is not available normally");
    assert!(matches!(
        rejection,
        Rejection::NotAllowed {
            event: ArticleEvent::ForcePublish,
            from: ArticleState::Draft,
        }
    ));

    let outcome = engine
        .execute_from_any(&article, ArticleEvent::ForcePublish, &())
        .expect("from_any can use the edge without origin");

    assert_eq!(outcome.transition.to, ArticleState::Published);
    assert!(outcome.transition.from_any);
    assert!(outcome.entity.published);
    assert_eq!(
        outcome.transition.rules,
        vec![
            "ForcePublish",
            "always",
            "entering_published",
            "leaving_draft",
            "from_any",
        ]
    );
    assert_eq!(
        effects(&outcome),
        vec![
            "fx:always",
            "fx:entering_published",
            "fx:leaving_draft",
            "fx:from_any",
        ]
    );
}

#[test]
fn diagnose_from_any_matches_execute_from_any() {
    let engine = article_engine();
    let article = article_in(ArticleState::Draft, false);

    let diagnosis = engine
        .diagnose_from_any(&article, ArticleEvent::ForcePublish, &())
        .expect("from_any diagnosis is valid");

    assert_eq!(diagnosis.to, ArticleState::Published);
    assert!(diagnosis.from_any);
    assert_eq!(
        diagnosis.rules,
        vec![
            "ForcePublish",
            "always",
            "entering_published",
            "leaving_draft",
            "from_any",
        ]
    );
}

#[test]
fn available_events_are_the_ones_with_an_origin_in_the_current_state() {
    let engine = article_engine();
    let article = article_in(ArticleState::Review, false);

    assert_eq!(
        engine.available(&article),
        &[
            ArticleEvent::Approve,
            ArticleEvent::Reject,
            ArticleEvent::Retry,
            ArticleEvent::Publish,
        ]
    );
    assert!(
        engine
            .available_detailed(&article, &())
            .iter()
            .all(|(_, verdict)| verdict.is_ok())
    );
}

#[test]
fn mermaid_shows_the_edge_without_origin_as_a_note() {
    let diagram = article_engine().to_mermaid();

    assert!(diagram.contains("    [*] --> Draft"), "{diagram}");
    assert!(
        diagram.contains("    note right of Published : ForcePublish"),
        "{diagram}"
    );
}
