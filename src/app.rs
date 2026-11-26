use self::context::AppContext;

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

pub mod context;

/// Global application state
pub enum AppState {
    Uninitialized,
    Running {
        window: Arc<Window>,
        context: AppContext,
    },
}

impl AppState {
    /// Returns 'true' if the state is an [`Uninitialized`] value
    pub fn is_uninitialized(&self) -> bool {
        matches!(*self, AppState::Uninitialized)
    }

    /// Returns 'true' if the state is a [`Running`] value
    pub fn is_running(&self) -> bool {
        matches!(*self, AppState::Running { .. })
    }
}

/// Main application
pub struct App {
    /// Application title
    title: &'static str,
    /// Global application state
    state: AppState,
    /// Application window is occluded flag
    is_occluded: bool,
}

impl App {
    /// Instantiate a new application
    pub fn new(title: &'static str) -> Self {
        Self {
            title,
            state: AppState::Uninitialized,
            is_occluded: false,
        }
    }
}

impl ApplicationHandler for App {
    /// Application start or resume event handler
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize application
        if self.state.is_uninitialized() {
            // Get existing application window, or create a new one
            let window = match event_loop
                .create_window(Window::default_attributes().with_title(self.title))
            {
                Ok(window) => Arc::new(window),
                Err(e) => {
                    log::error!("{e}");
                    event_loop.exit();
                    return;
                }
            };

            // Create app context
            let context = match AppContext::init(window.clone()) {
                Ok(context) => context,
                Err(e) => {
                    log::error!("{e}");
                    event_loop.exit();
                    return;
                }
            };

            self.state = AppState::Running { window, context };
        }

        // Start or resume application
        if let AppState::Running { ref window, .. } = self.state {
            window.request_redraw();
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let AppState::Running {
            ref window,
            ref mut context,
            ..
        } = self.state
        else {
            // Don't process event if application is not initialized
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                // Don't render when application is occluded
                if self.is_occluded {
                    return;
                }

                match context.render(window.clone()) {
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                        return;
                    }
                    _ => {}
                };

                // Continue render loop
                window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                context.resize(size);
                window.request_redraw();
            }
            WindowEvent::Occluded(is_occluded) => {
                self.is_occluded = is_occluded;

                // Resume rendering when unoccluded
                if !is_occluded {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            }
            | WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

/// Application runner
pub struct AppRunner {
    app: App,
}

impl AppRunner {
    /// Create a new runner for a provided app
    pub fn new(app: App) -> Self {
        Self::builder(app).build()
    }

    /// Create a new app runner builder
    pub fn builder(app: App) -> AppRunnerBuilder {
        AppRunnerBuilder::new(app)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        log::debug!(
            "Enabled wgpu backends: {:?}",
            wgpu::Instance::enabled_backend_features()
        );

        let event_loop = EventLoop::with_user_event().build()?;

        log::info!("Starting application event loop...");
        event_loop.run_app(&mut self.app)?;

        Ok(())
    }
}

/// Application runner builder
pub struct AppRunnerBuilder {
    app: App,
}

impl AppRunnerBuilder {
    /// Create a new app runner builder
    pub fn new(app: App) -> Self {
        Self { app }
    }

    /// Initialize environment logger
    pub fn with_logger(self, level: log::LevelFilter) -> Self {
        env_logger::builder()
            .filter_level(level)
            .format_timestamp_millis()
            .parse_default_env()
            .init();

        self
    }

    /// Build a new app runner
    pub fn build(self) -> AppRunner {
        AppRunner { app: self.app }
    }
}
