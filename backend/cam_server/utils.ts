///This file contains utility functions and types that are used throughout the project
//Stealing Rust's Result type for some basic stuff.
// Define the generic Result type
export type Result<T, E> = Ok<T> | Err<E>;

// Success variant
export type Ok<T> = {
    kind: 'ok';
    value: T;
};

// Error variant
export type Err<E> = {
    kind: 'err';
    error: E;
};

// Functions to help create Ok and Err types
export function ok<T>(value: T): Ok<T> {
    return { kind: 'ok', value };
}

export function err<E>(error: E): Err<E> {
    return { kind: 'err', error };
}

// Example usage:
/*
function divide(a: number, b: number): Result<number, string> {
    if (b === 0) {
        return err('Cannot divide by zero');
    } else {
        return ok(a / b);
    }
}

const result = divide(4, 2);

if (result.kind === 'ok') {
    console.log('Result:', result.value);
} else {
    console.error('Error:', result.error);
}
*/