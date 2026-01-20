use crate::{
    grid::Position,
    materials::ItemName,
    workers::tasks::{Priority, TaskTarget},
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Configuration parameters for the dispatcher's behavior.
#[derive(Resource)]
pub struct DispatcherConfig {
    /// How early (in seconds) to dispatch workers for predicted pickups.
    pub prediction_buffer_secs: f32,
    /// Maximum distance (in cells) to consider for task chaining.
    pub chain_distance_threshold: i32,
    /// Minimum urgency required to chain a task.
    pub chain_urgency_threshold: f32,
    /// Urgency level at which priority gets elevated.
    pub urgency_elevation_threshold: f32,
    /// Target number of production cycles to maintain in input buffers.
    pub target_input_cycles: u32,
}

impl Default for DispatcherConfig {
    fn default() -> Self {
        Self {
            prediction_buffer_secs: 0.5,
            chain_distance_threshold: 5,
            chain_urgency_threshold: 0.3,
            urgency_elevation_threshold: 0.8,
            target_input_cycles: 10,
        }
    }
}

/// Central dispatcher resource that manages worker assignments globally.
#[derive(Resource, Default)]
pub struct WorkerDispatcher {
    /// Pending logistics requests waiting for worker assignment.
    pub pending_requests: Vec<DispatchRequest>,
    /// Predicted future pickups based on production progress.
    pub predicted_pickups: Vec<PredictedPickup>,
    /// Workers currently pooled at the hub, available for immediate dispatch.
    pub pooled_workers: HashSet<Entity>,
}

impl WorkerDispatcher {
    /// Adds a new dispatch request to the pending queue.
    pub fn add_request(&mut self, request: DispatchRequest) {
        self.pending_requests.push(request);
    }

    /// Removes all requests associated with a specific building.
    pub fn remove_requests_for_building(&mut self, building: Entity) {
        self.pending_requests
            .retain(|req| req.source != building && req.destination != building);
    }

    /// Clears all pending requests and predictions.
    pub fn clear(&mut self) {
        self.pending_requests.clear();
        self.predicted_pickups.clear();
    }

    /// Registers a worker as available in the pool.
    pub fn pool_worker(&mut self, worker: Entity) {
        self.pooled_workers.insert(worker);
    }

    /// Removes a worker from the pool (when assigned).
    pub fn unpool_worker(&mut self, worker: Entity) {
        self.pooled_workers.remove(&worker);
    }

    /// Returns whether a worker is currently pooled.
    pub fn is_worker_pooled(&self, worker: Entity) -> bool {
        self.pooled_workers.contains(&worker)
    }

    /// Adds a predicted pickup.
    pub fn add_prediction(&mut self, prediction: PredictedPickup) {
        self.predicted_pickups.push(prediction);
    }

    /// Claims a prediction for a worker.
    pub fn claim_prediction(&mut self, building: Entity, worker: Entity) -> bool {
        for prediction in &mut self.predicted_pickups {
            if prediction.building == building && prediction.claimed_by.is_none() {
                prediction.claimed_by = Some(worker);
                return true;
            }
        }
        false
    }

    /// Releases any predictions claimed by a worker.
    pub fn release_claims(&mut self, worker: Entity) {
        for prediction in &mut self.predicted_pickups {
            if prediction.claimed_by == Some(worker) {
                prediction.claimed_by = None;
            }
        }
    }

    /// Removes stale predictions (already ready or building changed state).
    pub fn cleanup_predictions(&mut self, current_time: f32) {
        self.predicted_pickups
            .retain(|p| p.ready_at > current_time - 1.0);
    }
}

/// A logistics request awaiting worker assignment.
#[derive(Clone, Debug)]
pub struct DispatchRequest {
    /// The building to pick up from.
    pub source: Entity,
    /// The building to deliver to.
    pub destination: Entity,
    /// Items to transfer.
    pub items: HashMap<ItemName, u32>,
    /// Task priority level.
    pub priority: Priority,
    /// Urgency score from 0.0 (low) to 1.0 (critical).
    pub urgency: f32,
    /// Grid position of the source building.
    pub source_pos: Position,
    /// Grid position of the destination building.
    pub destination_pos: Position,
}

impl DispatchRequest {
    /// Creates a new dispatch request with default urgency.
    pub fn new(
        source: Entity,
        source_pos: Position,
        destination: Entity,
        destination_pos: Position,
        items: HashMap<ItemName, u32>,
        priority: Priority,
    ) -> Self {
        Self {
            source,
            destination,
            items,
            priority,
            urgency: 0.0,
            source_pos,
            destination_pos,
        }
    }

    /// Creates a request with specified urgency.
    pub fn with_urgency(mut self, urgency: f32) -> Self {
        self.urgency = urgency.clamp(0.0, 1.0);
        self
    }

    /// Returns the effective priority, potentially elevated by urgency.
    pub fn effective_priority(&self, config: &DispatcherConfig) -> Priority {
        if self.urgency >= config.urgency_elevation_threshold {
            match self.priority {
                Priority::Low => Priority::Medium,
                Priority::Medium => Priority::High,
                Priority::High | Priority::Critical => Priority::Critical,
            }
        } else {
            self.priority.clone()
        }
    }
}

/// A predicted future pickup based on production progress.
#[derive(Clone, Debug)]
pub struct PredictedPickup {
    /// The building that will have items ready.
    pub building: Entity,
    /// Grid position of the building.
    pub position: Position,
    /// Game time when items will be ready for pickup.
    pub ready_at: f32,
    /// Items that will be produced.
    pub items: HashMap<ItemName, u32>,
    /// Worker entity if already dispatched.
    pub claimed_by: Option<Entity>,
}

impl PredictedPickup {
    /// Creates a new prediction.
    pub fn new(
        building: Entity,
        position: Position,
        ready_at: f32,
        items: HashMap<ItemName, u32>,
    ) -> Self {
        Self {
            building,
            position,
            ready_at,
            items,
            claimed_by: None,
        }
    }

    /// Returns whether this prediction is unclaimed.
    pub fn is_available(&self) -> bool {
        self.claimed_by.is_none()
    }
}

/// Clears dispatcher state when tasks are bulk-cleared (F5 debug key).
pub fn clear_dispatcher_on_task_clear(
    keys: Res<ButtonInput<KeyCode>>,
    mut dispatcher: ResMut<WorkerDispatcher>,
) {
    if keys.just_pressed(KeyCode::F5) {
        dispatcher.clear();
        println!("Dispatcher: Cleared all pending requests and predictions");
    }
}

/// Removes dispatch requests for buildings that no longer have active tasks.
pub fn cleanup_stale_requests(
    mut dispatcher: ResMut<WorkerDispatcher>,
    existing_targets: Query<&TaskTarget>,
) {
    let target_entities: HashSet<Entity> = existing_targets.iter().map(|t| t.0).collect();

    let initial_count = dispatcher.pending_requests.len();
    dispatcher
        .pending_requests
        .retain(|req| !target_entities.contains(&req.source));

    let removed = initial_count - dispatcher.pending_requests.len();
    if removed > 0 {
        println!("Dispatcher: Removed {removed} stale requests");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn mock_entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn mock_position(x: i32, y: i32) -> Position {
        Position { x, y }
    }

    #[test]
    fn dispatcher_default_is_empty() {
        let dispatcher = WorkerDispatcher::default();

        assert!(dispatcher.pending_requests.is_empty());
        assert!(dispatcher.predicted_pickups.is_empty());
        assert!(dispatcher.pooled_workers.is_empty());
    }

    #[test]
    fn add_request_stores_request() {
        let mut dispatcher = WorkerDispatcher::default();
        let request = DispatchRequest::new(
            mock_entity(1),
            mock_position(0, 0),
            mock_entity(2),
            mock_position(5, 5),
            HashMap::new(),
            Priority::Medium,
        );

        dispatcher.add_request(request);

        assert_eq!(dispatcher.pending_requests.len(), 1);
    }

    #[test]
    fn remove_requests_for_building_removes_matching() {
        let mut dispatcher = WorkerDispatcher::default();
        let building = mock_entity(1);

        dispatcher.add_request(DispatchRequest::new(
            building,
            mock_position(0, 0),
            mock_entity(2),
            mock_position(5, 5),
            HashMap::new(),
            Priority::Medium,
        ));
        dispatcher.add_request(DispatchRequest::new(
            mock_entity(3),
            mock_position(10, 10),
            building,
            mock_position(0, 0),
            HashMap::new(),
            Priority::Medium,
        ));
        dispatcher.add_request(DispatchRequest::new(
            mock_entity(4),
            mock_position(1, 1),
            mock_entity(5),
            mock_position(2, 2),
            HashMap::new(),
            Priority::Medium,
        ));

        dispatcher.remove_requests_for_building(building);

        assert_eq!(dispatcher.pending_requests.len(), 1);
        assert_eq!(dispatcher.pending_requests[0].source, mock_entity(4));
    }

    #[test]
    fn pool_and_unpool_worker() {
        let mut dispatcher = WorkerDispatcher::default();
        let worker = mock_entity(10);

        dispatcher.pool_worker(worker);
        assert!(dispatcher.is_worker_pooled(worker));

        dispatcher.unpool_worker(worker);
        assert!(!dispatcher.is_worker_pooled(worker));
    }

    #[test]
    fn claim_prediction_marks_as_claimed() {
        let mut dispatcher = WorkerDispatcher::default();
        let building = mock_entity(1);
        let worker = mock_entity(10);

        dispatcher.add_prediction(PredictedPickup::new(
            building,
            mock_position(5, 5),
            10.0,
            HashMap::new(),
        ));

        let claimed = dispatcher.claim_prediction(building, worker);
        assert!(claimed);
        assert_eq!(dispatcher.predicted_pickups[0].claimed_by, Some(worker));
    }

    #[test]
    fn claim_prediction_fails_if_already_claimed() {
        let mut dispatcher = WorkerDispatcher::default();
        let building = mock_entity(1);
        let worker1 = mock_entity(10);
        let worker2 = mock_entity(11);

        dispatcher.add_prediction(PredictedPickup::new(
            building,
            mock_position(5, 5),
            10.0,
            HashMap::new(),
        ));

        dispatcher.claim_prediction(building, worker1);
        let second_claim = dispatcher.claim_prediction(building, worker2);

        assert!(!second_claim);
        assert_eq!(dispatcher.predicted_pickups[0].claimed_by, Some(worker1));
    }

    #[test]
    fn release_claims_frees_predictions() {
        let mut dispatcher = WorkerDispatcher::default();
        let building = mock_entity(1);
        let worker = mock_entity(10);

        dispatcher.add_prediction(PredictedPickup::new(
            building,
            mock_position(5, 5),
            10.0,
            HashMap::new(),
        ));
        dispatcher.claim_prediction(building, worker);

        dispatcher.release_claims(worker);

        assert!(dispatcher.predicted_pickups[0].claimed_by.is_none());
    }

    #[test]
    fn effective_priority_elevates_at_high_urgency() {
        let config = DispatcherConfig::default();
        let mut request = DispatchRequest::new(
            mock_entity(1),
            mock_position(0, 0),
            mock_entity(2),
            mock_position(5, 5),
            HashMap::new(),
            Priority::Medium,
        );

        request.urgency = 0.5;
        assert_eq!(request.effective_priority(&config), Priority::Medium);

        request.urgency = 0.9;
        assert_eq!(request.effective_priority(&config), Priority::High);
    }

    #[test]
    fn urgency_clamped_to_valid_range() {
        let request = DispatchRequest::new(
            mock_entity(1),
            mock_position(0, 0),
            mock_entity(2),
            mock_position(5, 5),
            HashMap::new(),
            Priority::Medium,
        )
        .with_urgency(1.5);

        assert!((request.urgency - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn predicted_pickup_is_available_when_unclaimed() {
        let prediction =
            PredictedPickup::new(mock_entity(1), mock_position(5, 5), 10.0, HashMap::new());

        assert!(prediction.is_available());
    }
}
