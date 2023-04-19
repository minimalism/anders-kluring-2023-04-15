use bevy::prelude::*;
use kayak_ui::prelude::{widgets::*, KStyle, *, kayak_font::Alignment};

use crate::kluring::RestartEvent;

use super::BoardState;

pub struct ShowUiPlugin;

impl Plugin for ShowUiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PreloadResource>()
            .add_plugin(KayakContextPlugin)
            .add_plugin(KayakWidgets)
            .add_startup_system(startup_gui)
        ;
    }
}

// ----- draw board state string -----

#[derive(Component, Default, Clone, PartialEq, Eq)]
pub struct StateWidgetProps {
    pub area_x: i32,
    pub area_y: i32,
    pub attempts: usize,
}

fn statewidget_render(
    In((_widget_context, entity)): In<(KayakWidgetContext, Entity)>,
    board_state: Res<BoardState>,
    mut query: Query<(&mut StateWidgetProps, &KStyle, &mut ComputedStyles)>,
) -> bool {
    if let Ok((mut w, style, mut computed_styles)) = query.get_mut(entity) {
        if board_state.bounds.is_default() {
            w.area_x = 0;
            w.area_y = 0;
            w.attempts = 0;
        } else {
            w.area_x = board_state.bounds.width();
            w.area_y = board_state.bounds.height();
            w.attempts = board_state.attempts;
        }

        // Note: We will see two updates because of the mutable change to styles.
        // Which means when foo changes MyWidget will render twice!
        *computed_styles = KStyle {
            render_command: StyleProp::Value(RenderCommand::Text {
                content: format!("Area: {} ({} * {}) ({} attempts)", w.area_x * w.area_y, w.area_x, w.area_y, w.attempts),
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
pub struct StateWidgetBundle {
    props: StateWidgetProps,
    styles: KStyle,
    computed_styles: ComputedStyles,
    widget_name: WidgetName,
}

impl Default for StateWidgetBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: StateWidgetProps::default().get_name(),
        }
    }
}

// ----- input fields -----

#[derive(Component, Default, Clone, PartialEq)]
struct TextBoxExample;

#[derive(Component, Default, Clone, PartialEq)]
pub struct InputFieldsState {
    pub n: String,
    pub value2: String,
}

impl Widget for TextBoxExample {}

#[derive(Bundle)]
struct TextBoxExampleBundle {
    text_box_example: TextBoxExample,
    styles: KStyle,
    widget_name: WidgetName,
}

impl Default for TextBoxExampleBundle {
    fn default() -> Self {
        Self {
            text_box_example: Default::default(),
            styles: Default::default(),
            widget_name: TextBoxExample::default().get_name(),
        }
    }
}

fn update_input_fields(
    In((widget_context, entity)): In<(KayakWidgetContext, Entity)>,
    mut commands: Commands,
    state_query: Query<&InputFieldsState>,
) -> bool {
    let state_entity = widget_context.use_state::<InputFieldsState>(
        &mut commands,
        entity,
        InputFieldsState {
            n: "1".into(),
            value2: "?".into(),
        },
    );

    if let Ok(textbox_state) = state_query.get(state_entity) {
        let on_change_n = OnChange::new(
            move |In((_widget_context, _, value)): In<(KayakWidgetContext, Entity, String)>,
                  mut state_query: Query<&mut InputFieldsState>| {
                if let Ok(mut state) = state_query.get_mut(state_entity) {
                    state.n = value;
                }
            },
        );

        let on_change2 = OnChange::new(
            move |In((_widget_context, _, value)): In<(KayakWidgetContext, Entity, String)>,
                  mut state_query: Query<&mut InputFieldsState>| {
                if let Ok(mut state) = state_query.get_mut(state_entity) {
                    state.value2 = value;
                }
            },
        );

        let parent_id = Some(entity);
        rsx! {
            <ElementBundle>
                <TextBoxBundle
                    styles={KStyle {
                        bottom: StyleProp::Value(Units::Pixels(10.0)),
                        ..Default::default()
                    }}
                    text_box={TextBoxProps { value: textbox_state.n.clone(), ..Default::default()}}
                    on_change={on_change_n}
                />
                <TextBoxBundle
                    text_box={TextBoxProps { value: textbox_state.value2.clone(), ..Default::default()}}
                    on_change={on_change2}
                />
            </ElementBundle>
        };
    }
    true
}

// ----- draw buttons -----

#[derive(Default, Clone, PartialEq, Component)]
pub struct MenuButton {
    text: String,
}

impl Widget for MenuButton {}

#[derive(Bundle)]
pub struct MenuButtonBundle {
    button: MenuButton,
    styles: KStyle,
    on_event: OnEvent,
    widget_name: WidgetName,
}

impl Default for MenuButtonBundle {
    fn default() -> Self {
        Self {
            button: Default::default(),
            styles: KStyle {
                bottom: Units::Pixels(20.0).into(),
                cursor: KCursorIcon(CursorIcon::Hand).into(),
                ..Default::default()
            },
            on_event: OnEvent::default(),
            widget_name: MenuButton::default().get_name(),
        }
    }
}

fn menu_button_render(
    In((widget_context, entity)): In<(KayakWidgetContext, Entity)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu_button_query: Query<&MenuButton>,
    state_query: Query<&ButtonState>,
) -> bool {
    let state_entity =
        widget_context.use_state(&mut commands, entity, ButtonState { hovering: false });

    let button_text = menu_button_query.get(entity).unwrap().text.clone();
    let button_image = asset_server.load("main_menu/button.png");
    let button_image_hover = asset_server.load("main_menu/button-hover.png");

    let on_event = OnEvent::new(
        move |In((event_dispatcher_context, _, mut event, _entity)): In<(
            EventDispatcherContext,
            WidgetState,
            KEvent,
            Entity,
        )>,
              mut query: Query<&mut ButtonState>| {
            if let Ok(mut button) = query.get_mut(state_entity) {
                match event.event_type {
                    EventType::MouseIn(..) => {
                        event.stop_propagation();
                        button.hovering = true;
                    }
                    EventType::MouseOut(..) => {
                        button.hovering = false;
                    }
                    _ => {}
                }
            }
            (event_dispatcher_context, event)
        },
    );

    if let Ok(button_state) = state_query.get(state_entity) {
        let button_image_handle = if button_state.hovering {
            button_image_hover
        } else {
            button_image
        };

        let parent_id = Some(entity);
        rsx! {
            <NinePatchBundle
                nine_patch={NinePatch {
                    handle: button_image_handle,
                    border: Edge::all(10.0),
                }}
                styles={KStyle {
                    width: Units::Stretch(1.0).into(),
                    height: Units::Pixels(40.0).into(),
                    bottom: Units::Pixels(30.0).into(),
                    left: Units::Pixels(50.0).into(),
                    right: Units::Pixels(50.0).into(),
                    ..KStyle::default()
                }}
                on_event={on_event}
            >
                <TextWidgetBundle
                    styles={KStyle {
                        top: Units::Stretch(1.0).into(),
                        bottom: Units::Stretch(1.0).into(),
                        ..Default::default()
                    }}
                    text={TextProps {
                        alignment: Alignment::Middle,
                        content: button_text,
                        size: 28.0,
                        ..Default::default()
                    }}
                />
            </NinePatchBundle>
        };
    }
    true
}

#[derive(Default, Resource)]
pub struct PreloadResource {
    images: Vec<Handle<Image>>,
}

// ----- initialization -----

fn startup_gui(
    mut commands: Commands,
    mut font_mapping: ResMut<FontMapping>,
    asset_server: Res<AssetServer>,
    mut preload_resource: ResMut<PreloadResource>,
) {
    let camera_entity = commands
        .spawn((Camera2dBundle::default(), CameraUIKayak))
        .id();

    font_mapping.set_default(asset_server.load("roboto.kayak_font"));

    let mut widget_context = KayakRootContext::new(camera_entity);
    widget_context.add_plugin(KayakWidgetsContextPlugin);

    widget_context.add_widget_data::<MenuButton, ButtonState>();
    widget_context.add_widget_system(
        MenuButton::default().get_name(),
        widget_update::<MenuButton, ButtonState>,
        menu_button_render,
    );

    widget_context.add_widget_data::<StateWidgetProps, EmptyState>();
    widget_context.add_widget_system(
        StateWidgetProps::default().get_name(),
        widget_update_with_resource::<StateWidgetProps, EmptyState>,
        statewidget_render,
    );

    let panel1_image = asset_server.load("panel1.png");
    let button_image = asset_server.load("button.png");
    let button_image_hover = asset_server.load("button-hover.png");

    preload_resource.images.extend(vec![
        button_image.clone(),
        button_image_hover.clone(),
    ]);

    let handle_click_close = OnEvent::new(
        move |In((event_dispatcher_context, _, event, _entity)): In<(
            EventDispatcherContext,
            WidgetState,
            KEvent,
            Entity,
        )>,
        mut restart: EventWriter<RestartEvent>| {
            match event.event_type {
                EventType::Click(..) => {
                    restart.send(RestartEvent { });
                }
                _ => {}
            }
            (event_dispatcher_context, event)
        },
    );

    widget_context.add_widget_data::<TextBoxExample, InputFieldsState>();
    widget_context.add_widget_system(
        TextBoxExample::default().get_name(),
        widget_update::<TextBoxExample, InputFieldsState>,
        update_input_fields,
    );

    let parent_id = None;

    rsx! {
        <KayakAppBundle>
            <NinePatchBundle
                nine_patch={NinePatch {
                    handle: panel1_image,
                    border: Edge::all(25.0),
                }}
                styles={KStyle {
                    width: Units::Pixels(400.0).into(),
                    height: Units::Pixels(200.0).into(),
                    left: Units::Stretch(0.0).into(),
                    right: Units::Stretch(1.0).into(),
                    top: Units::Stretch(1.0).into(),
                    bottom: Units::Stretch(1.0).into(),
                    padding: Edge::new(
                        Units::Pixels(20.0),
                        Units::Pixels(20.0),
                        Units::Pixels(50.0),
                        Units::Pixels(20.0),
                    ).into(),
                    ..KStyle::default()
                }}
                >

                <StateWidgetBundle props={
                    StateWidgetProps { 
                        area_x: 0,
                        area_y: 0,
                        attempts: 0,
                    }
                } />

                <TextBoxExampleBundle />

                <MenuButtonBundle
                    button={MenuButton { text: "Restart".into() }}
                    on_event={handle_click_close}
                />

            </NinePatchBundle>
        </KayakAppBundle>
    };

    commands.spawn((widget_context, EventDispatcher::default()));
}
