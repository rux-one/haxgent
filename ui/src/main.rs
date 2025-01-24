mod logger;

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::{collections::HashMap, io, thread::current};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::logger::Logger;

enum AgentState {
    Idle,
    Thinking,
    Working,
}

struct AgentConfig {
    host: String,
}

struct Agent {
    state: AgentState,
    config: AgentConfig,
}

impl Agent {
    fn new() -> Self {
        Self {
            state: AgentState::Idle,
            config: AgentConfig {
                host: String::from("127.0.0.1"),
            },
        }
    }

    fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }

    fn get_state(&self) -> &AgentState {
        &self.state
    }

    fn set_host(&mut self, host: String) {
        self.config.host = host;
    }

    fn get_host(&self) -> &String {
        &self.config.host
    }
}

struct Commands {
    commands: HashMap<String, String>,
    current_command: String,
    current_command_args: Vec<String>,
}

impl Commands {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
            current_command: String::new(),
            current_command_args: vec![],
        }
    }

    fn add_command(&mut self, command: String, description: String) {
        self.commands.insert(command, description);
    }

    fn setup_commands(&self) -> HashMap<String, String> {
        let mut commands = HashMap::new();
        commands.insert("sethost".to_string(), "Set host".to_string());
        commands.insert("help".to_string(), "Available commands: hello, quit, sethost".to_string());
        commands.insert("quit".to_string(), "Goodbye!".to_string());
        commands
    }

    fn capture_command(&mut self, input: &str) {
        if input.is_empty() {
            return;
        }

        let cmd = input.split(" ").collect::<Vec<&str>>()[0];
        let args = input.split(" ").collect::<Vec<&str>>()[1..].join(" ");

        self.current_command = String::from(cmd);
        self.current_command_args = args.split(" ").map(|s| s.to_string()).collect();
    }

    fn get_current_command(&self) -> &String {
        &self.current_command
    }

    fn get_current_command_args(&self) -> &Vec<String> {
        &self.current_command_args
    }

    fn reset_command(&mut self) {
        self.current_command = String::new();
        self.current_command_args = vec![];
    }
}




fn main() -> Result<(), io::Error> {
    let mut commands = Commands::new();
    let mut agent = Agent::new();

    let mut log: Logger = Logger::new();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    
    let mut input = String::new();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .constraints([Constraint::Max(3), Constraint::Max(5), Constraint::Max(1)].as_ref())
                .split(size);

            // Display command output
            commands.capture_command(&input);
            let output_widget = Paragraph::new(log.format_logs())
                .block(Block::default().borders(Borders::ALL).title(" -- log --"));
            f.render_widget(output_widget, chunks[2]);

            let settings = format!("Target host: {}", agent.get_host());
            let settings_widget = Paragraph::new(settings)
                .block(Block::default().borders(Borders::ALL).title(" -- settings -- "));
            f.render_widget(settings_widget, chunks[1]);
            // Input area
            let input_widget = Paragraph::new(input.as_ref() as &str)
                .block(Block::default().borders(Borders::ALL).title(" -- command --"));
            f.render_widget(input_widget, chunks[0]);
        })?;

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(500))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Enter => {
                        // Execute command on Enter key

                        match commands.get_current_command().as_str() {
                            "sethost" => {
                                agent.set_host(commands.get_current_command_args()[0].clone());
                                log.add(
                                    format!("Set host to {}", agent.get_host()),
                                    format!("Host changed to {}", agent.get_host())
                                );
                            },
                            "exit" => break,
                            _ => {}
                        }

                        input.clear();  // Clear input after execution
                    }
                    crossterm::event::KeyCode::Esc => break,  // Exit on Esc key
                    crossterm::event::KeyCode::Char(c) => {
                        input.push(c);  // Add character to input
                    }
                    crossterm::event::KeyCode::Backspace => {
                        input.pop();  // Remove last character on Backspace
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
