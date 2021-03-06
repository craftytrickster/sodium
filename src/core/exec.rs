use state::editor::Editor;
use io::parse::{Inst, Parameter};
use state::mode::{Mode, CommandMode, PrimitiveMode};
use edit::insert::{InsertOptions, InsertMode};
use io::redraw::RedrawTask;
use edit::buffer::Buffer;

// TODO: Move the command definitions outta here
impl Editor {
    /// Execute an instruction
    pub fn exec(&mut self, Inst(para, cmd): Inst) {
        use io::key::Key::*;
        use state::mode::Mode::*;
        use state::mode::PrimitiveMode::*;
        use state::mode::CommandMode::*;

        let n = para.d();
        let bef = self.pos();
        let mut mov = false;

        match (self.cursor().mode, cmd.key) {
            (Primitive(Prompt), Char(' ')) if self.key_state.shift => {
                self.prompt = String::new();
                self.cursor_mut().mode = Mode::Command(CommandMode::Normal);
            },
            (Primitive(Insert(_)), Char(' ')) if self.key_state.shift => {
                let left = self.left(1);
                self.goto(left);
                self.cursor_mut().mode = Mode::Command(CommandMode::Normal);
            },
            (_, Char(' ')) if self.key_state.shift =>
                self.cursor_mut().mode = Mode::Command(CommandMode::Normal),
            (_, Char(' ')) if self.key_state.alt => self.next_cursor(),
            _ if self.key_state.alt => if let Some(m) = self.to_motion(Inst(para, cmd)) {
                self.goto(m);
            },
            (Command(Normal), Char('i')) => {
                self.cursor_mut().mode =
                    Mode::Primitive(PrimitiveMode::Insert(InsertOptions {
                        mode: InsertMode::Insert,
                    }));

            }
            (Command(Normal), Char('a')) => {
                let pos = self.right(1, false);
                self.goto( pos );
                self.cursor_mut().mode =
                    Mode::Primitive(PrimitiveMode::Insert(InsertOptions {
                        mode: InsertMode::Insert,
                    }));

            }
            (Command(Normal), Char('o')) => {
                let y = self.y();
                let ind = if self.options.autoindent {
                    self.buffer.get_indent(y).to_owned()
                } else {
                    String::new()
                };
                let last = ind.len();
                self.buffer.insert_line(y, ind.into());
                self.goto((last, y + 1));
                self.cursor_mut().mode =
                    Mode::Primitive(PrimitiveMode::Insert(InsertOptions {
                        mode: InsertMode::Insert,
                    }));
            }
            (Command(Normal), Char('h')) => {
                let left = self.left(n);
                self.goto(left);
                mov = true;
            }
            (Command(Normal), Char('j')) => {
                let down = self.down(n);
                self.goto(down);
                mov = true;
            }
            (Command(Normal), Char('k')) => {
                let up = self.up(n);
                self.goto(up);
                mov = true;
            }
            (Command(Normal), Char('l')) => {
                let right = self.right(n, true);
                self.goto(right);
                mov = true;
            }
            (Command(Normal), Char('J')) => {
                let down = self.down(15 * n);
                self.goto(down);
                mov = true;
            }
            (Command(Normal), Char('K')) => {
                let up = self.up(15 * n);
                self.goto(up);
                mov = true;
            }
            (Command(Normal), Char('x')) => {
                self.delete();
                let bounded = self.bound(self.pos(), true);
                self.goto(bounded);
            }
            (Command(Normal), Char('X')) => {
                self.backspace();
                let bounded = self.bound(self.pos(), true);
                self.goto(bounded);
            }
            (Command(Normal), Char('L')) => {
                let ln_end = (self.buffer[self.y()].len(), self.y());
                self.goto(ln_end);
                mov = true;
            }
            (Command(Normal), Char('H')) => {
                self.cursor_mut().x = 0;
                mov = true;
            }
            (Command(Normal), Char('r')) => {
                let (x, y) = self.pos();
                let c = self.get_char();
                // If there is nothing in the current buffer
                // ignore the command
                if self.buffer[y].len() > 0 {
                    self.buffer[y].remove(x);
                }
                self.buffer[y].insert(x, c);
            }
            (Command(Normal), Char('R')) => {
                self.cursor_mut().mode =
                    Mode::Primitive(PrimitiveMode::Insert(InsertOptions {
                        mode: InsertMode::Replace,
                    }));
            }
            (Command(Normal), Char('d')) => {
                let ins = self.get_inst();
                if let Some(m) = self.to_motion_unbounded(ins) {
                    self.remove_rb(m);
                }
            }
            (Command(Normal), Char('G')) => {
                let last = self.buffer.len() - 1;
                self.goto((0, last));
                mov = true;
            }
            (Command(Normal), Char('g')) => {
                if let Parameter::Int(n) = para {
                    self.goto((0, n - 1));
                    mov = true;
                } else {
                    let inst = self.get_inst();
                    if let Some(m) = self.to_motion(inst) {
                        self.goto(m); // fix
                        mov = true;
                    }
                }

            }
            (Command(Normal), Char('b')) => {
                // Branch cursor
                if self.cursors.len() < 255 {
                    let cursor = self.cursor().clone();
                    self.cursors.insert(self.current_cursor as usize, cursor);
                    self.next_cursor();
                }
                else {
                    self.status_bar.msg = format!("At max 255 cursors");
                }
            }
            (Command(Normal), Char('B')) => {
                // Delete cursor
                if self.cursors.len() > 1 {
                    self.cursors.remove(self.current_cursor as usize);
                    self.prev_cursor();
                }
                else {
                    self.status_bar.msg = format!("No other cursors!");
                }
            }
            (Command(Normal), Char('t')) => {
                let ch = self.get_char();

                let pos = self.next_ocur(ch, n);
                if let Some(p) = pos {
                    let y = self.y();
                    self.goto((p, y));
                    mov = true;
                }
            }
            (Command(Normal), Char('f')) => {
                let ch = self.get_char();

                let pos = self.previous_ocur(ch, n);
                if let Some(p) = pos {
                    let y = self.y();
                    self.goto((p, y));
                    mov = true;
                }
            }
            (Command(Normal), Char(';')) =>
                self.cursor_mut().mode = Mode::Primitive(PrimitiveMode::Prompt),
            (Command(Normal), Char(' ')) => self.next_cursor(),
            (Command(Normal), Char('z')) => {
                let Inst(param, cmd) = self.get_inst();
                match param {
                    Parameter::Null => {
                        if let Some(m) = self.to_motion(Inst(param, cmd)) {
                            self.scroll_y = m.1;
                            self.goto(m);
                        }
                    }
                    Parameter::Int(n) => {
                        self.scroll_y = n;
                    }
                }
                self.redraw_task = RedrawTask::Full;
            }
            (Command(Normal), Char('Z')) => {
                self.scroll_y = self.y() - 3;
                self.redraw_task = RedrawTask::Full;
            }
            (Command(Normal), Char('~')) => {
                self.invert_chars(n);
            }
            (Command(Normal), Char(c)) => {
                self.status_bar.msg = format!("Unknown command: {}", c);
                self.redraw_task = RedrawTask::StatusBar;
            }
            (Primitive(Insert(opt)), k) => self.insert(k, opt),
            (Primitive(Prompt), Char('\n')) => {
                self.cursor_mut().mode = Command(Normal);
                let cmd = self.prompt.clone();

                self.invoke(cmd);
                self.prompt = String::new();
                self.redraw_task = RedrawTask::StatusBar;
            },
            (Primitive(Prompt), Backspace) => {
                self.prompt.pop();
                self.redraw_task = RedrawTask::StatusBar;
            },
            (Primitive(Prompt), Char(c)) => {
                self.prompt.push(c);
                self.redraw_task = RedrawTask::StatusBar;
            },
            _ => {
                self.status_bar.msg = format!("Unknown command");
                self.redraw_task = RedrawTask::StatusBar;
            },
        }
        if mov {
            self.redraw_task = RedrawTask::Cursor(bef, self.pos());
        }
    }
}
