use std::{
    ops::{
        Range,
        Deref,
        DerefMut,
    },
    fmt::Display,
    marker::PhantomData,
};
pub use error::*;


mod error;


pub struct GenericSubParser<'source,'borrow,Kind:Display+EOFError> {
    parent:&'borrow mut GenericParser<'source,Kind>,
    error:bool,
}
impl<'source,'borrow,Kind:Display+EOFError> GenericSubParser<'source,'borrow,Kind> {
    pub fn new(parent:&'borrow mut GenericParser<'source,Kind>)->Self {
        parent.stack.push(parent.index);
        Self {
            parent,
            error:true,
        }
    }
    /// Saves the index and removes the old one.
    pub fn finish(mut self) {self.error=false}
    /// Restores the old index.
    pub fn finish_error(mut self) {self.error=true}
}
impl<'source,'borrow,Kind:Display+EOFError> Deref for GenericSubParser<'source,'borrow,Kind> {
    type Target=GenericParser<'source,Kind>;
    fn deref(&self)->&Self::Target {
        self.parent
    }
}
impl<'source,'borrow,Kind:Display+EOFError> DerefMut for GenericSubParser<'source,'borrow,Kind> {
    fn deref_mut(&mut self)->&mut Self::Target {
        self.parent
    }
}
impl<'source,'borrow,Kind:Display+EOFError> Drop for GenericSubParser<'source,'borrow,Kind> {
    fn drop(&mut self) {
        if self.error { // Reset the index
            self.parent.index=self.parent.stack.pop().unwrap_or(self.parent.index);
        } else {        // Remove the pushed index. We don't need it anymore
            self.parent.stack.pop();
        }
    }
}
#[derive(Debug,Clone)]
pub struct GenericParser<'source,Kind:Display+EOFError> {
    source:&'source str,
    stack:Vec<usize>,
    index:usize,
    filename:&'source str,
    _phantom:PhantomData<Kind>,
}
impl<'source,Kind:Display+EOFError> GenericParser<'source,Kind> {
    pub fn new(source:&'source str,filename:&'source str)->Self {
        Self {
            source,
            stack:Vec::new(),
            index:0,
            filename,
            _phantom:PhantomData,
        }
    }
    pub fn create_error_with_suberrors(&self,kind:Kind,important:bool,suberrors:Vec<Error<'source,Kind>>)->Error<'source,Kind> {
        Error{
            line:self.get_line_range(),
            column:self.get_column_range(),
            kind,
            filename:self.filename(),
            suberrors,
            important,
        }
    }
    pub fn create_error(&self,kind:Kind,important:bool)->Error<'source,Kind> {
        Error{
            line:self.get_line_range(),
            column:self.get_column_range(),
            kind,
            filename:self.filename(),
            suberrors:Vec::new(),
            important,
        }
    }
    #[inline]#[must_use]pub fn subparser<'borrow>(&'borrow mut self)->GenericSubParser<'source,'borrow,Kind> {GenericSubParser::new(self)}
    #[inline]pub fn eof_error(&self)->Error<'source,Kind> {self.create_error(Kind::create_eof(),true)}
    #[inline]pub fn is_eof(&self)->bool {self.source[self.index..].len()==0}
    #[inline]pub fn filename(&self)->&'source str {self.filename}
    #[inline]pub fn level(&self)->usize {self.stack.len()}
    #[inline]pub fn source_left(&self)->&str {&self.source[self.index..]}
    #[inline]pub fn get_source(&self)->&str {&self.source}
    pub fn eat(&mut self,count:usize)->Result<&'source str,Error<'source,Kind>> {
        let start=self.index;
        for _ in 0..count {
            if self.is_eof() {
                return Err(self.eof_error());
            }
            self.set_next_char_boundary();
        }
        return Ok(&self.source[start..self.index]);
    }
    pub fn get_line_range(&self)->Range<usize> {
        let l=self.get_line();
        return l..l;
    }
    pub fn get_column_range(&self)->Range<usize> {
        let c=self.get_column();
        return c..c;
    }
    pub fn get_line(&self)->usize {
        if self.index==0 {return 1}
        return self.source[..self.index].lines().count();
    }
    pub fn get_column(&self)->usize {
        if self.index==0 {return 0}
        let mut last=None;
        for s in self.source[..self.index].lines() {
            last=Some(s);
        }
        let column=last.map(|s|s.chars().count()).unwrap_or(1).saturating_sub(1);
        return column;
    }
    pub fn test(&self,s:&str)->Result<bool,Error<'source,Kind>> {
        if self.is_eof() {
            return Err(self.eof_error());
        }
        if self.source[self.index..].starts_with(s) {
            return Ok(true);
        }
        return Ok(false);
    }
    pub fn test_any(&self,options:&[&str])->Result<bool,Error<'source,Kind>> {
        for option in options {
            if self.test(option)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    pub fn then(&mut self,s:&str)->Result<bool,Error<'source,Kind>> {
        if self.test(s)? {
            self.index+=s.len();
            return Ok(true);
        }
        return Ok(false);
    }
    pub fn then_any(&mut self,options:&[&str])->Result<bool,Error<'source,Kind>> {
        for option in options {
            if self.then(option)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }
    pub fn skip(&mut self,options:&[&str])->&mut Self {
        loop {
            match self.then_any(options) {
                Ok(true)=>continue,
                _=>{},
            }
            break;
        }
        return self;
    }
    fn set_next_char_boundary(&mut self) {
        self.index+=1;
        while !self.source.is_char_boundary(self.index) {self.index+=1}
    }
    pub fn until(&mut self,ending:&str)->&'source str {
        let start=self.index;
        'main:loop {
            match self.test(ending) {
                Ok(true)|Err(_)=>break 'main,
                _=>{},
            }
            self.set_next_char_boundary();
        }
        return &self.source[start..self.index];
    }
    pub fn until_any(&mut self,endings:&[&str])->&'source str {
        let start=self.index;
        loop {
            match self.test_any(endings) {
                Ok(true)|Err(_)=>break,
                _=>{},
            }
            self.set_next_char_boundary();
        }
        return &self.source[start..self.index];
    }
    pub fn until_including(&mut self,endings:&[&str],exceptions:&[&str])->&'source str {
        let start=self.index;
        'main:loop {
            for option in exceptions {
                if let Err(_)=self.then(option) {
                    break 'main;
                }
            }
            for option in endings {
                let res=self.test(option);
                match res {
                    Ok(true)|Err(_)=>break 'main,
                    _=>{},
                }
            }
            self.set_next_char_boundary();
        }
        return &self.source[start..self.index];
    }
    pub fn while_any(&mut self,options:&[&str])->&'source str {
        let start=self.index;
        'main:loop {
            for option in options {
                match self.then(option) {
                    Ok(true)=>continue 'main,
                    Ok(false)=>{},
                    Err(_)=>break 'main,
                }
            }
            break;
        }
        return &self.source[start..self.index];
    }
    pub fn while_any_delimited(&mut self,options:&[&str],delimiters:&[&str])->&'source str {
        let start=self.index;
        let mut before_delimiter=self.index;
        let mut has_delimiter=false;
        'delimiter:loop {
            let start2=self.index;
            for option in options {
                match self.then(option) {
                    Ok(true)=>break,
                    Ok(false)=>{},
                    Err(_)=>break 'delimiter,
                }
            }
            if start2==self.index {
                self.index=before_delimiter;
                break 'delimiter;
            }
            before_delimiter=self.index;
            for delimiter in delimiters {
                match self.then(delimiter) {
                    Ok(true)=>{
                        has_delimiter=true;
                        continue 'delimiter;
                    },
                    Ok(false)=>{},
                    Err(_)=>break 'delimiter,
                }
            }
            if !has_delimiter {
                break 'delimiter;
            }
        }
        return &self.source[start..self.index];
    }
    pub fn while_any_delimited_counted(&mut self,options:&[&str],delimiters:&[&str],max_delimiters:usize)->&'source str {
        let start=self.index;
        let mut before_delimiter=self.index;
        let mut has_delimiter=false;
        let mut count=0;
        'delimiter:loop {
            let start2=self.index;
            for option in options {
                match self.then(option) {
                    Ok(true)=>break,
                    Ok(false)=>{},
                    Err(_)=>break 'delimiter,
                }
            }
            if count>=max_delimiters {
                break 'delimiter;
            }
            if start2==self.index {
                self.index=before_delimiter;
                break 'delimiter;
            }
            before_delimiter=self.index;
            for delimiter in delimiters {
                match self.then(delimiter) {
                    Ok(true)=>{
                        count+=1;
                        has_delimiter=true;
                        continue 'delimiter;
                    },
                    Ok(false)=>{},
                    Err(_)=>break 'delimiter,
                }
            }
            if !has_delimiter {
                break 'delimiter;
            }
        }
        return &self.source[start..self.index];
    }
}
