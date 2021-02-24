#[macro_export]
macro_rules! ternary {
    ($c:stmt, $v1:expr, $v2:expr) => {
        if $c {
            return $v1
        } else {
            return $v2
        }
    };
}

#[macro_export]
macro_rules! if_let_some {
    ($var:pat = $value:expr) => {
        let $var = if let Some(it) = $value {
            it
        } else {
            return;
        };
    };

    ($var:pat = $value:expr, $else_value:expr) => {
        #[allow(clippy::if_let_some_result)]
        let $var = if let Some(it) = $value {
            it
        } else {
            return $else_value;
        };
    }
}

#[macro_export]
macro_rules! if_let_ok {
    ($var:pat = $value:expr, $else_value:expr) => {
        let $var = match $value {
            Ok(it) => it,
            Err(err) => return $else_value(err),
        };
    }
}

#[macro_export]
macro_rules! try_except_return {
    ($connection_statement:expr, $msg:literal) => {
        match $connection_statement {
            Ok(value) => value,
            Err(e) => {
                error!(crate::LOGGER, "{}: {}", $msg, e);
                return;
            },
        }
    }
}