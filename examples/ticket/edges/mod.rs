//! One type per transition.

mod agent_assigned;
mod agent_reassigned;
mod customer_replied;
mod direct_ticket_created;
mod fix_deployed;
mod fix_rejected;
mod inactivity_timeout;
mod info_requested;
mod new_diagnosis_started;
mod new_draft;
mod solution_confirmed;
mod supervisor_override;
mod ticket_submitted;
mod work_started;

pub use agent_assigned::AgentAssigned;
pub use agent_reassigned::AgentReassigned;
pub use customer_replied::CustomerReplied;
pub use direct_ticket_created::DirectTicketCreated;
pub use fix_deployed::FixDeployed;
pub use fix_rejected::FixRejected;
pub use inactivity_timeout::InactivityTimeout;
pub use info_requested::InfoRequested;
pub use new_diagnosis_started::NewDiagnosisStarted;
pub use new_draft::NewDraft;
pub use solution_confirmed::SolutionConfirmed;
pub use supervisor_override::SupervisorOverride;
pub use ticket_submitted::TicketSubmitted;
pub use work_started::WorkStarted;
