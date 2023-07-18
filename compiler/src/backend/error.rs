pub type CompilispResult<T> = Result<T, CompilispError>;

#[derive(Debug)]
pub enum CompilispError {
    IllFormedSyntax,
}
