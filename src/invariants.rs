use bevy::prelude::*;

use crate::{
    grid::Position,
    materials::Cargo,
    structures::{Building, BuildingCost, ConstructionSite},
    systems::Operational,
    workers::{Speed, Worker, WorkerPath, WorkflowAssignment},
};

pub struct InvariantPlugin;

impl Plugin for InvariantPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                check_worker_components,
                check_building_components,
                check_workflow_references,
                check_exclusive_worker_states,
                check_construction_site_components,
            ),
        );
    }
}

fn report_violation(entity: Entity, message: &str) {
    let msg = format!("INVARIANT VIOLATION [{entity:?}]: {message}");
    if cfg!(test) {
        #[allow(clippy::panic)]
        {
            panic!("{msg}");
        }
    } else {
        error!("{msg}");
    }
}

fn check_worker_components(
    workers: Query<
        (
            Entity,
            Has<Speed>,
            Has<Position>,
            Has<WorkerPath>,
            Has<Cargo>,
            Has<crate::structures::ComputeConsumer>,
            Has<Transform>,
        ),
        With<Worker>,
    >,
) {
    for (entity, has_speed, has_position, has_path, has_cargo, has_compute, has_transform) in
        &workers
    {
        if !has_speed {
            report_violation(entity, "worker missing Speed component");
        }
        if !has_position {
            report_violation(entity, "worker missing Position component");
        }
        if !has_path {
            report_violation(entity, "worker missing WorkerPath component");
        }
        if !has_cargo {
            report_violation(entity, "worker missing Cargo component");
        }
        if !has_compute {
            report_violation(entity, "worker missing ComputeConsumer component");
        }
        if !has_transform {
            report_violation(entity, "worker missing Transform component");
        }
    }
}

fn check_building_components(
    buildings: Query<
        (
            Entity,
            Has<Name>,
            Has<Position>,
            Has<Operational>,
            Has<Transform>,
        ),
        With<Building>,
    >,
) {
    for (entity, has_name, has_position, has_operational, has_transform) in &buildings {
        if !has_name {
            report_violation(entity, "building missing Name component");
        }
        if !has_position {
            report_violation(entity, "building missing Position component");
        }
        if !has_operational {
            report_violation(entity, "building missing Operational component");
        }
        if !has_transform {
            report_violation(entity, "building missing Transform component");
        }
    }
}

fn check_workflow_references(
    assignments: Query<(Entity, &WorkflowAssignment)>,
    workflows: Query<&crate::workers::workflows::Workflow>,
) {
    for (entity, assignment) in &assignments {
        if workflows.get(assignment.workflow).is_err() {
            report_violation(
                entity,
                &format!(
                    "WorkflowAssignment references dead workflow {:?}",
                    assignment.workflow
                ),
            );
        }
    }
}

fn check_exclusive_worker_states(
    workers: Query<
        (
            Entity,
            Has<crate::workers::workflows::WaitingForItems>,
            Has<crate::workers::workflows::WaitingForSpace>,
        ),
        With<Worker>,
    >,
) {
    for (entity, has_waiting_items, has_waiting_space) in &workers {
        if has_waiting_items && has_waiting_space {
            report_violation(
                entity,
                "worker has both WaitingForItems and WaitingForSpace",
            );
        }
    }
}

fn check_construction_site_components(
    sites: Query<
        (
            Entity,
            Has<crate::materials::InputPort>,
            Has<BuildingCost>,
            Has<Position>,
            Has<Transform>,
        ),
        With<ConstructionSite>,
    >,
) {
    for (entity, has_input, has_cost, has_position, has_transform) in &sites {
        if !has_input {
            report_violation(entity, "construction site missing InputPort component");
        }
        if !has_cost {
            report_violation(entity, "construction site missing BuildingCost component");
        }
        if !has_position {
            report_violation(entity, "construction site missing Position component");
        }
        if !has_transform {
            report_violation(entity, "construction site missing Transform component");
        }
    }
}
