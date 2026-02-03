use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

#[derive(Component)]
pub struct Scrollable;

const SCROLL_LINE_HEIGHT: f32 = 21.0;

#[allow(clippy::cast_precision_loss)]
pub fn handle_ui_scroll(
    mut mouse_wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    mut scroll_query: Query<
        (
            &mut ScrollPosition,
            &UiGlobalTransform,
            &ComputedNode,
            &Node,
            &Children,
        ),
        With<Scrollable>,
    >,
    child_sizes: Query<&ComputedNode, Without<Scrollable>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    for scroll in mouse_wheel.read() {
        let delta = match scroll.unit {
            MouseScrollUnit::Line => scroll.y * SCROLL_LINE_HEIGHT,
            MouseScrollUnit::Pixel => scroll.y,
        };

        for (mut scroll_pos, ui_transform, container_node, container_style, children) in
            &mut scroll_query
        {
            let center = ui_transform.translation;
            let half = container_node.size() / 2.0;
            if cursor_pos.x < center.x - half.x
                || cursor_pos.x > center.x + half.x
                || cursor_pos.y < center.y - half.y
                || cursor_pos.y > center.y + half.y
            {
                continue;
            }

            let content_height: f32 = children
                .iter()
                .filter_map(|child| child_sizes.get(child).ok())
                .map(|node| node.size().y)
                .sum();

            let gap = match container_style.row_gap {
                Val::Px(px) => px,
                _ => 0.0,
            };
            let gap_total = children.len().saturating_sub(1) as f32 * gap;
            let total_content = content_height + gap_total;
            let max_offset = (total_content - container_node.size().y).max(0.0);
            scroll_pos.y = (scroll_pos.y - delta).clamp(0.0, max_offset);
        }
    }
}
