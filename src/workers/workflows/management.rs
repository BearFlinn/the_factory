use bevy::prelude::*;

use crate::{grid::Position, workers::Worker};

use super::components::{
    AssignWorkersEvent, BatchAssignWorkersEvent, CreateWorkflowEvent, DeleteWorkflowEvent,
    PauseWorkflowEvent, UnassignWorkersEvent, Workflow, WorkflowAssignment, WorkflowRegistry,
};

pub fn handle_create_workflow(
    mut commands: Commands,
    mut events: MessageReader<CreateWorkflowEvent>,
    mut registry: ResMut<WorkflowRegistry>,
) {
    for event in events.read() {
        let entity = commands
            .spawn(Workflow {
                name: event.name.clone(),
                building_set: event.building_set.clone(),
                steps: event.steps.clone(),
                is_paused: false,
                desired_worker_count: event.desired_worker_count,
            })
            .id();
        registry.workflows.push(entity);
    }
}

pub fn handle_delete_workflow(
    mut commands: Commands,
    mut events: MessageReader<DeleteWorkflowEvent>,
    mut registry: ResMut<WorkflowRegistry>,
    assignments: Query<(Entity, &WorkflowAssignment)>,
) {
    for event in events.read() {
        commands.entity(event.workflow).despawn();
        registry.workflows.retain(|&e| e != event.workflow);

        for (worker_entity, assignment) in &assignments {
            if assignment.workflow == event.workflow {
                commands
                    .entity(worker_entity)
                    .remove::<WorkflowAssignment>();
            }
        }
    }
}

pub fn handle_pause_workflow(
    mut events: MessageReader<PauseWorkflowEvent>,
    mut workflows: Query<&mut Workflow>,
) {
    for event in events.read() {
        if let Ok(mut workflow) = workflows.get_mut(event.workflow) {
            workflow.is_paused = !workflow.is_paused;
        }
    }
}

pub fn handle_assign_workers(
    mut commands: Commands,
    mut events: MessageReader<AssignWorkersEvent>,
) {
    for event in events.read() {
        for &worker in &event.workers {
            commands.entity(worker).insert(WorkflowAssignment {
                workflow: event.workflow,
                current_step: 0,
                resolved_target: None,
            });
        }
    }
}

pub fn handle_unassign_workers(
    mut commands: Commands,
    mut events: MessageReader<UnassignWorkersEvent>,
) {
    for event in events.read() {
        for &worker in &event.workers {
            commands.entity(worker).remove::<WorkflowAssignment>();
        }
    }
}

pub fn handle_batch_assign_workers(
    mut events: MessageReader<BatchAssignWorkersEvent>,
    workflows: Query<&Workflow>,
    idle_workers: Query<(Entity, &Position), (With<Worker>, Without<WorkflowAssignment>)>,
    assigned_workers: Query<&WorkflowAssignment, With<Worker>>,
    positions: Query<&Position>,
    mut commands: Commands,
) {
    for event in events.read() {
        let Ok(workflow) = workflows.get(event.workflow) else {
            continue;
        };

        let current_assigned = assigned_workers
            .iter()
            .filter(|a| a.workflow == event.workflow)
            .count();

        #[allow(clippy::cast_possible_truncation)]
        let needed = (event.count as usize).saturating_sub(current_assigned);
        if needed == 0 {
            continue;
        }

        if workflow.building_set.is_empty() {
            continue;
        }

        let (sum_x, sum_y, count) =
            workflow
                .building_set
                .iter()
                .fold((0i64, 0i64, 0u32), |(sx, sy, c), &entity| {
                    if let Ok(pos) = positions.get(entity) {
                        (sx + i64::from(pos.x), sy + i64::from(pos.y), c + 1)
                    } else {
                        (sx, sy, c)
                    }
                });

        if count == 0 {
            continue;
        }

        #[allow(clippy::cast_possible_truncation)]
        let centroid_x = (sum_x / i64::from(count)) as i32;
        #[allow(clippy::cast_possible_truncation)]
        let centroid_y = (sum_y / i64::from(count)) as i32;

        let mut candidates: Vec<(Entity, i32)> = idle_workers
            .iter()
            .map(|(entity, pos)| {
                let dist = (pos.x - centroid_x).abs() + (pos.y - centroid_y).abs();
                (entity, dist)
            })
            .collect();

        candidates.sort_by_key(|&(_, dist)| dist);

        for (worker_entity, _) in candidates.into_iter().take(needed) {
            commands.entity(worker_entity).insert(WorkflowAssignment {
                workflow: event.workflow,
                current_step: 0,
                resolved_target: None,
            });
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::workers::workflows::components::{StepTarget, WorkflowAction, WorkflowStep};

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_message::<CreateWorkflowEvent>();
        app.add_message::<DeleteWorkflowEvent>();
        app.add_message::<PauseWorkflowEvent>();
        app.add_message::<AssignWorkersEvent>();
        app.add_message::<UnassignWorkersEvent>();
        app.add_message::<BatchAssignWorkersEvent>();
        app.init_resource::<WorkflowRegistry>();
        app.add_systems(
            Update,
            (
                handle_create_workflow,
                handle_delete_workflow,
                handle_pause_workflow,
                handle_assign_workers,
                handle_unassign_workers,
                handle_batch_assign_workers,
            ),
        );
        app
    }

    #[test]
    fn create_workflow_spawns_entity_and_adds_to_registry() {
        let mut app = setup_app();

        app.world_mut().write_message(CreateWorkflowEvent {
            name: "test workflow".to_string(),
            building_set: HashSet::new(),
            steps: vec![WorkflowStep {
                target: StepTarget::Specific(Entity::PLACEHOLDER),
                action: WorkflowAction::Pickup(None),
            }],
            desired_worker_count: 2,
        });
        app.update();

        let registry = app.world().resource::<WorkflowRegistry>();
        assert_eq!(registry.workflows.len(), 1);

        let workflow_entity = registry.workflows[0];
        let workflow = app.world().get::<Workflow>(workflow_entity).unwrap();
        assert_eq!(workflow.name, "test workflow");
        assert_eq!(workflow.steps.len(), 1);
        assert!(!workflow.is_paused);
        assert_eq!(workflow.desired_worker_count, 2);
    }

    #[test]
    fn delete_workflow_despawns_and_removes_from_registry() {
        let mut app = setup_app();

        app.world_mut().write_message(CreateWorkflowEvent {
            name: "to delete".to_string(),
            building_set: HashSet::new(),
            steps: vec![],
            desired_worker_count: 1,
        });
        app.update();

        let workflow_entity = app.world().resource::<WorkflowRegistry>().workflows[0];

        app.world_mut().write_message(DeleteWorkflowEvent {
            workflow: workflow_entity,
        });
        app.update();

        let registry = app.world().resource::<WorkflowRegistry>();
        assert!(registry.workflows.is_empty());
        assert!(app.world().get_entity(workflow_entity).is_err());
    }

    #[test]
    fn delete_workflow_unassigns_workers() {
        let mut app = setup_app();

        app.world_mut().write_message(CreateWorkflowEvent {
            name: "worker workflow".to_string(),
            building_set: HashSet::new(),
            steps: vec![],
            desired_worker_count: 1,
        });
        app.update();

        let workflow_entity = app.world().resource::<WorkflowRegistry>().workflows[0];
        let worker_entity = app.world_mut().spawn_empty().id();

        app.world_mut().write_message(AssignWorkersEvent {
            workflow: workflow_entity,
            workers: vec![worker_entity],
        });
        app.update();

        assert!(app
            .world()
            .get::<WorkflowAssignment>(worker_entity)
            .is_some());

        app.world_mut().write_message(DeleteWorkflowEvent {
            workflow: workflow_entity,
        });
        app.update();

        assert!(app
            .world()
            .get::<WorkflowAssignment>(worker_entity)
            .is_none());
    }

    #[test]
    fn pause_workflow_toggles() {
        let mut app = setup_app();

        app.world_mut().write_message(CreateWorkflowEvent {
            name: "pausable".to_string(),
            building_set: HashSet::new(),
            steps: vec![],
            desired_worker_count: 1,
        });
        app.update();

        let workflow_entity = app.world().resource::<WorkflowRegistry>().workflows[0];

        let workflow = app.world().get::<Workflow>(workflow_entity).unwrap();
        assert!(!workflow.is_paused);

        app.world_mut().write_message(PauseWorkflowEvent {
            workflow: workflow_entity,
        });
        app.update();

        let workflow = app.world().get::<Workflow>(workflow_entity).unwrap();
        assert!(workflow.is_paused);

        app.world_mut().write_message(PauseWorkflowEvent {
            workflow: workflow_entity,
        });
        app.update();

        let workflow = app.world().get::<Workflow>(workflow_entity).unwrap();
        assert!(!workflow.is_paused);
    }

    #[test]
    fn assign_workers_adds_assignment() {
        let mut app = setup_app();

        let workflow_entity = app
            .world_mut()
            .spawn(Workflow {
                name: "assign test".to_string(),
                building_set: HashSet::new(),
                steps: vec![],
                is_paused: false,
                desired_worker_count: 2,
            })
            .id();

        let worker_a = app.world_mut().spawn_empty().id();
        let worker_b = app.world_mut().spawn_empty().id();

        app.world_mut().write_message(AssignWorkersEvent {
            workflow: workflow_entity,
            workers: vec![worker_a, worker_b],
        });
        app.update();

        let assignment_a = app.world().get::<WorkflowAssignment>(worker_a).unwrap();
        assert_eq!(assignment_a.workflow, workflow_entity);
        assert_eq!(assignment_a.current_step, 0);

        let assignment_b = app.world().get::<WorkflowAssignment>(worker_b).unwrap();
        assert_eq!(assignment_b.workflow, workflow_entity);
        assert_eq!(assignment_b.current_step, 0);
    }

    #[test]
    fn unassign_workers_removes_assignment() {
        let mut app = setup_app();

        let workflow_entity = app
            .world_mut()
            .spawn(Workflow {
                name: "unassign test".to_string(),
                building_set: HashSet::new(),
                steps: vec![],
                is_paused: false,
                desired_worker_count: 1,
            })
            .id();

        let worker = app.world_mut().spawn_empty().id();

        app.world_mut().write_message(AssignWorkersEvent {
            workflow: workflow_entity,
            workers: vec![worker],
        });
        app.update();

        assert!(app.world().get::<WorkflowAssignment>(worker).is_some());

        app.world_mut().write_message(UnassignWorkersEvent {
            workers: vec![worker],
        });
        app.update();

        assert!(app.world().get::<WorkflowAssignment>(worker).is_none());
    }

    #[test]
    fn batch_assign_workers_picks_nearest() {
        let mut app = setup_app();

        let building = app.world_mut().spawn(Position { x: 10, y: 10 }).id();

        let mut building_set = HashSet::new();
        building_set.insert(building);

        let workflow_entity = app
            .world_mut()
            .spawn(Workflow {
                name: "batch test".to_string(),
                building_set,
                steps: vec![],
                is_paused: false,
                desired_worker_count: 2,
            })
            .id();

        let near_worker = app
            .world_mut()
            .spawn((Worker, Position { x: 9, y: 9 }))
            .id();
        let far_worker = app
            .world_mut()
            .spawn((Worker, Position { x: 100, y: 100 }))
            .id();

        app.world_mut().write_message(BatchAssignWorkersEvent {
            workflow: workflow_entity,
            count: 1,
        });
        app.update();

        assert!(app.world().get::<WorkflowAssignment>(near_worker).is_some());
        assert!(app.world().get::<WorkflowAssignment>(far_worker).is_none());
    }
}
