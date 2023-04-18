use bevy::prelude::*;
use kayak_ui::prelude::{widgets::*, KStyle, *, kayak_font::Alignment};

use super::{BoardState, Bounds};

pub struct ShowUiPlugin;

impl Plugin for ShowUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugin(KayakContextPlugin)
            .add_plugin(KayakWidgets)
            .add_startup_system(startup_gui)
        ;
    }
}

#[derive(Component, Default, Clone, PartialEq, Eq)]
pub struct StateWidgetProps {
    pub area_x: i32,
    pub area_y: i32,
}


fn statewidget_render(
    In((_widget_context, entity)): In<(KayakWidgetContext, Entity)>,
    board_state: Res<BoardState>,
    mut query: Query<(&mut StateWidgetProps, &KStyle, &mut ComputedStyles)>,
) -> bool {
    if let Ok((mut my_widget, style, mut computed_styles)) = query.get_mut(entity) {
        if board_state.bounds.is_default() {
            my_widget.area_x = 0;
            my_widget.area_y = 0;
        } else {
            my_widget.area_x = board_state.bounds.max_x - board_state.bounds.min_x;
            my_widget.area_y = board_state.bounds.max_y - board_state.bounds.min_y;
        }

        // Note: We will see two updates because of the mutable change to styles.
        // Which means when foo changes MyWidget will render twice!
        *computed_styles = KStyle {
            render_command: StyleProp::Value(RenderCommand::Text {
                content: format!("Area: {} ({} * {})", my_widget.area_x * my_widget.area_y, my_widget.area_x, my_widget.area_y),
                alignment: Alignment::Start,
                word_wrap: false,
                subpixel: false,
            }),
            ..Default::default()
        }
        .with_style(style)
        .into();
    }

    true
}

// Our own version of widget_update that handles resource change events.
pub fn widget_update_with_resource<
    Props: PartialEq + Component + Clone,
    State: PartialEq + Component + Clone,
>(
    In((widget_context, entity, previous_entity)): In<(KayakWidgetContext, Entity, Entity)>,
    my_resource: Res<BoardState>,
    widget_param: WidgetParam<Props, State>,
) -> bool {
    widget_param.has_changed(&widget_context, entity, previous_entity) || my_resource.is_changed()
}

impl Widget for StateWidgetProps {}

#[derive(Bundle)]
pub struct MyWidgetBundle {
    props: StateWidgetProps,
    styles: KStyle,
    computed_styles: ComputedStyles,
    widget_name: WidgetName,
}

impl Default for MyWidgetBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: StateWidgetProps::default().get_name(),
        }
    }
}

fn startup_gui(
    mut commands: Commands,
    mut font_mapping: ResMut<FontMapping>,
    asset_server: Res<AssetServer>,
) {
    let camera_entity = commands
        .spawn((Camera2dBundle::default(), CameraUIKayak))
        .id();

    font_mapping.set_default(asset_server.load("roboto.kayak_font"));

    let mut widget_context = KayakRootContext::new(camera_entity);
    widget_context.add_plugin(KayakWidgetsContextPlugin);
    let parent_id = None;
    widget_context.add_widget_data::<StateWidgetProps, EmptyState>();
    widget_context.add_widget_system(
        StateWidgetProps::default().get_name(),
        widget_update_with_resource::<StateWidgetProps, EmptyState>,
        statewidget_render,
    );
    rsx! {
        <KayakAppBundle><MyWidgetBundle props={StateWidgetProps { area_x: 0, area_y: 0 }} /></KayakAppBundle>
    };

    commands.spawn((widget_context, EventDispatcher::default()));
}
