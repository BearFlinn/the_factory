use bevy::prelude::*;

use crate::structures::construction::{
    NeedsRecipeCommitmentEvaluation, RecipeCommitment, RecipeCrafter,
};

#[cfg(test)]
use bevy::ecs::system::SystemState;

pub fn any_needs_evaluation(query: Query<(), With<NeedsRecipeCommitmentEvaluation>>) -> bool {
    !query.is_empty()
}

pub fn evaluate_recipe_commitments(
    mut commands: Commands,
    mut query: Query<
        (Entity, &RecipeCrafter, &mut RecipeCommitment),
        With<NeedsRecipeCommitmentEvaluation>,
    >,
) {
    for (entity, crafter, mut commitment) in &mut query {
        commands
            .entity(entity)
            .remove::<NeedsRecipeCommitmentEvaluation>();

        let current_recipe = crafter.get_active_recipe().cloned();

        if current_recipe == commitment.committed_recipe {
            continue;
        }

        if commitment.can_commit_new_recipe() {
            commitment.committed_recipe = current_recipe;
        } else {
            commitment.pending_recipe = current_recipe;
        }
    }
}

pub fn commit_pending_recipes(mut query: Query<&mut RecipeCommitment>) {
    for mut commitment in &mut query {
        if commitment.pending_recipe.is_some() && commitment.can_commit_new_recipe() {
            commitment.committed_recipe = commitment.pending_recipe.clone();
            commitment.pending_recipe = None;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::materials::RecipeName;
    use std::collections::HashMap;

    #[test]
    fn any_needs_evaluation_returns_true_when_marker_present() {
        let mut app = App::new();

        app.world_mut().spawn(NeedsRecipeCommitmentEvaluation);

        let mut system_state: SystemState<Query<(), With<NeedsRecipeCommitmentEvaluation>>> =
            SystemState::new(app.world_mut());

        let query = system_state.get(app.world());
        assert!(any_needs_evaluation(query));
    }

    #[test]
    fn any_needs_evaluation_returns_false_when_no_marker() {
        let mut app = App::new();

        let mut system_state: SystemState<Query<(), With<NeedsRecipeCommitmentEvaluation>>> =
            SystemState::new(app.world_mut());

        let query = system_state.get(app.world());
        assert!(!any_needs_evaluation(query));
    }

    #[test]
    fn evaluate_recipe_commitments_removes_marker() {
        let mut app = App::new();

        let recipe_name: RecipeName = "Test Recipe".to_string();
        let crafter = RecipeCrafter {
            current_recipe: Some(recipe_name.clone()),
            available_recipes: Vec::new(),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let entity = app
            .world_mut()
            .spawn((
                crafter,
                RecipeCommitment::new_committed(Some(recipe_name)),
                NeedsRecipeCommitmentEvaluation,
            ))
            .id();

        let mut system_state: SystemState<(
            Commands,
            Query<
                (Entity, &RecipeCrafter, &mut RecipeCommitment),
                With<NeedsRecipeCommitmentEvaluation>,
            >,
        )> = SystemState::new(app.world_mut());

        let (commands, query) = system_state.get_mut(app.world_mut());
        evaluate_recipe_commitments(commands, query);
        system_state.apply(app.world_mut());

        assert!(!app
            .world()
            .entity(entity)
            .contains::<NeedsRecipeCommitmentEvaluation>());
    }

    #[test]
    fn evaluate_recipe_commitments_same_recipe_no_change() {
        let mut app = App::new();

        let recipe_name: RecipeName = "Test Recipe".to_string();
        let crafter = RecipeCrafter {
            current_recipe: Some(recipe_name.clone()),
            available_recipes: Vec::new(),
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let entity = app
            .world_mut()
            .spawn((
                crafter,
                RecipeCommitment::new_committed(Some(recipe_name.clone())),
                NeedsRecipeCommitmentEvaluation,
            ))
            .id();

        let mut system_state: SystemState<(
            Commands,
            Query<
                (Entity, &RecipeCrafter, &mut RecipeCommitment),
                With<NeedsRecipeCommitmentEvaluation>,
            >,
        )> = SystemState::new(app.world_mut());

        let (commands, query) = system_state.get_mut(app.world_mut());
        evaluate_recipe_commitments(commands, query);
        system_state.apply(app.world_mut());

        let commitment = app
            .world()
            .entity(entity)
            .get::<RecipeCommitment>()
            .cloned()
            .unwrap();
        assert_eq!(commitment.committed_recipe, Some(recipe_name));
        assert_eq!(commitment.pending_recipe, None);
    }

    #[test]
    fn evaluate_recipe_commitments_different_recipe_empty_transit_commits_immediately() {
        let mut app = App::new();

        let old_recipe: RecipeName = "Old Recipe".to_string();
        let new_recipe: RecipeName = "New Recipe".to_string();

        let crafter = RecipeCrafter {
            current_recipe: Some(new_recipe.clone()),
            available_recipes: vec![old_recipe.clone(), new_recipe.clone()],
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let entity = app
            .world_mut()
            .spawn((
                crafter,
                RecipeCommitment::new_committed(Some(old_recipe)),
                NeedsRecipeCommitmentEvaluation,
            ))
            .id();

        let mut system_state: SystemState<(
            Commands,
            Query<
                (Entity, &RecipeCrafter, &mut RecipeCommitment),
                With<NeedsRecipeCommitmentEvaluation>,
            >,
        )> = SystemState::new(app.world_mut());

        let (commands, query) = system_state.get_mut(app.world_mut());
        evaluate_recipe_commitments(commands, query);
        system_state.apply(app.world_mut());

        let commitment = app
            .world()
            .entity(entity)
            .get::<RecipeCommitment>()
            .cloned()
            .unwrap();
        assert_eq!(commitment.committed_recipe, Some(new_recipe));
        assert_eq!(commitment.pending_recipe, None);
    }

    #[test]
    fn evaluate_recipe_commitments_different_recipe_with_transit_sets_pending() {
        let mut app = App::new();

        let old_recipe: RecipeName = "Old Recipe".to_string();
        let new_recipe: RecipeName = "New Recipe".to_string();

        let crafter = RecipeCrafter {
            current_recipe: Some(new_recipe.clone()),
            available_recipes: vec![old_recipe.clone(), new_recipe.clone()],
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        };

        let mut commitment = RecipeCommitment::new_committed(Some(old_recipe.clone()));
        let mut in_transit = HashMap::new();
        in_transit.insert("Iron Ore".to_string(), 10);
        commitment.add_in_transit(&in_transit);

        let entity = app
            .world_mut()
            .spawn((crafter, commitment, NeedsRecipeCommitmentEvaluation))
            .id();

        let mut system_state: SystemState<(
            Commands,
            Query<
                (Entity, &RecipeCrafter, &mut RecipeCommitment),
                With<NeedsRecipeCommitmentEvaluation>,
            >,
        )> = SystemState::new(app.world_mut());

        let (commands, query) = system_state.get_mut(app.world_mut());
        evaluate_recipe_commitments(commands, query);
        system_state.apply(app.world_mut());

        let commitment = app
            .world()
            .entity(entity)
            .get::<RecipeCommitment>()
            .cloned()
            .unwrap();
        assert_eq!(commitment.committed_recipe, Some(old_recipe));
        assert_eq!(commitment.pending_recipe, Some(new_recipe));
    }

    #[test]
    fn commit_pending_recipes_commits_when_transit_empty() {
        let mut app = App::new();

        let old_recipe: RecipeName = "Old Recipe".to_string();
        let new_recipe: RecipeName = "New Recipe".to_string();

        let mut commitment = RecipeCommitment::new_committed(Some(old_recipe));
        commitment.pending_recipe = Some(new_recipe.clone());

        let entity = app.world_mut().spawn(commitment).id();

        let mut system_state: SystemState<Query<&mut RecipeCommitment>> =
            SystemState::new(app.world_mut());

        let query = system_state.get_mut(app.world_mut());
        commit_pending_recipes(query);
        system_state.apply(app.world_mut());

        let commitment = app
            .world()
            .entity(entity)
            .get::<RecipeCommitment>()
            .cloned()
            .unwrap();
        assert_eq!(commitment.committed_recipe, Some(new_recipe));
        assert_eq!(commitment.pending_recipe, None);
    }

    #[test]
    fn commit_pending_recipes_waits_when_transit_not_empty() {
        let mut app = App::new();

        let old_recipe: RecipeName = "Old Recipe".to_string();
        let new_recipe: RecipeName = "New Recipe".to_string();

        let mut commitment = RecipeCommitment::new_committed(Some(old_recipe.clone()));
        commitment.pending_recipe = Some(new_recipe.clone());
        let mut in_transit = HashMap::new();
        in_transit.insert("Iron Ore".to_string(), 10);
        commitment.add_in_transit(&in_transit);

        let entity = app.world_mut().spawn(commitment).id();

        let mut system_state: SystemState<Query<&mut RecipeCommitment>> =
            SystemState::new(app.world_mut());

        let query = system_state.get_mut(app.world_mut());
        commit_pending_recipes(query);
        system_state.apply(app.world_mut());

        let commitment = app
            .world()
            .entity(entity)
            .get::<RecipeCommitment>()
            .cloned()
            .unwrap();
        assert_eq!(commitment.committed_recipe, Some(old_recipe));
        assert_eq!(commitment.pending_recipe, Some(new_recipe));
    }
}
