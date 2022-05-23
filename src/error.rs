use std::{
    fmt::{
        Formatter,
        Display,
        Result as FmtResult,
        Debug,
    },
    ops::Range,
};


pub trait EOFErrorKind {
    fn create_eof()->Self;
}


pub struct EOFError;
impl Display for EOFError {
    fn fmt(&self,f:&mut Formatter)->FmtResult {
        write!(f,"Unexpected EOF")
    }
}
impl EOFErrorKind for EOFError {
    fn create_eof()->Self {EOFError}
}
#[derive(Debug)]
pub struct Error<'source,Kind:Display+EOFErrorKind=EOFError> {
    pub line:Range<usize>,
    pub column:Range<usize>,
    pub kind:Kind,
    pub filename:&'source str,
    pub suberrors:Vec<Self>,
    pub important:bool,
}
impl<'source,Kind:Display+EOFErrorKind> Error<'source,Kind> {
    pub fn print_with_context(&self,contents:&str,filter_important:bool) {
        self.print_with_context_prefix(contents,"",filter_important);
    }
    pub fn print_with_context_prefix(&self,contents:&str,prefix:impl Display,filter_important:bool) {
        //dbg!(&self.line,&self.column);
        let code_lines_vec=contents.lines().enumerate().collect::<Vec<_>>();
        let code_lines=&code_lines_vec[self.line.start.saturating_sub(1).min(code_lines_vec.len().saturating_sub(2))..=(self.line.end-1).min(code_lines_vec.len()-1)];
        let mut suberror_count=self.suberrors.iter().count();
        if filter_important {
            suberror_count=self.suberrors.iter().filter(|e|e.important).count();
        }
        println!("{}Error: {}",prefix,self.kind);
        println!("{}     ╭╴{}:{}:{}",prefix,self.filename,self.line.start,self.column.start+1);
        println!("{}     │",prefix);
        let mut to_remove=None;
        let to_add="… ";
        let mut to_add_len=0;
        let mut last_line_len:usize=0;
        for (num,line) in code_lines.iter() {
            let mut num_str=(num+1).to_string();
            while num_str.len()<5 {num_str.push(' ')}
            print!("{}{}│ ",prefix,num_str);
            if to_remove.is_none() {
                let trimmed=line.trim();
                // remove characters, but no more than the minimum column
                let mut remove_amount=line.chars().count()-trimmed.chars().count();
                if remove_amount>self.column.start {
                    remove_amount=self.column.start;
                }
                to_remove=Some(remove_amount);
                if remove_amount>0 {
                    to_add_len=to_add.chars().count();
                }
            }
            if let Some(to_remove)=to_remove {
                if to_remove>0 {
                    print!("{}{}",prefix,to_add);
                }
                let mut len=0;
                for c in line.chars().skip(to_remove) {
                    len+=1;
                    print!("{}",c);
                }
                last_line_len=len;
            }
            println!();
        }
        let to_remove=to_remove.unwrap_or(0);
        if suberror_count==0 {
            print!("{}     ╰─",prefix);
        } else {
            print!("{}     ├─",prefix);
        }
        let mut column=self.column.clone();
        let mut to=((column.start)+to_add_len)-to_remove;
        if column.start==usize::MAX {
            column.start=0;
            to=to_add_len;
        }
        if column.end==usize::MAX {
            column.end=last_line_len.saturating_sub(1);
        }
        for _ in 0..to {
            print!("─");
        }
        for _ in column {
            print!("┴");
        }
        println!("╯");
        if self.suberrors.len()>0 {
            if suberror_count>0 {
                println!("{}╭╴Or╶╯",prefix);
                if filter_important {
                    for (i,error) in self.suberrors.iter().filter(|e|e.important).enumerate() {
                        if i>0 {
                            println!("{}├╴Or",prefix);
                        }
                        error.print_with_context_prefix(contents,format!("{}│ ",prefix),filter_important);
                    }
                } else {
                    for (i,error) in self.suberrors.iter().enumerate() {
                        if i>0 {
                            println!("{}├╴Or",prefix);
                        }
                        error.print_with_context_prefix(contents,format!("{}│ ",prefix),filter_important);
                    }
                }
                println!("{}╰────╴",prefix);
            }
        }
    }
}
impl<'source,Kind:Display+EOFErrorKind> Display for Error<'source,Kind> {
    fn fmt(&self,f:&mut Formatter)->FmtResult {
        write!(f,"Error `{}` at ({}:{}) {}",self.kind,self.line.start,self.column.start,self.kind)
    }
}
