use bevy::prelude::*;

use super::components::{
    AssignWorkersEvent, CreateWorkflowEvent, DeleteWorkflowEvent, PauseWorkflowEvent,
    UnassignWorkersEvent, Workflow, WorkflowAssignment, WorkflowRegistry,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::workers::workflows::components::{WorkflowAction, WorkflowStep};

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_message::<CreateWorkflowEvent>();
        app.add_message::<DeleteWorkflowEvent>();
        app.add_message::<PauseWorkflowEvent>();
        app.add_message::<AssignWorkersEvent>();
        app.add_message::<UnassignWorkersEvent>();
        app.init_resource::<WorkflowRegistry>();
        app.add_systems(
            Update,
            (
                handle_create_workflow,
                handle_delete_workflow,
                handle_pause_workflow,
                handle_assign_workers,
                handle_unassign_workers,
            ),
        );
        app
    }

    #[test]
    fn create_workflow_spawns_entity_and_adds_to_registry() {
        let mut app = setup_app();

        app.world_mut().write_message(CreateWorkflowEvent {
            name: "test workflow".to_string(),
            steps: vec![WorkflowStep {
                target: Entity::PLACEHOLDER,
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
}
