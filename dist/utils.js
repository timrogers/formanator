import promptSync from 'prompt-sync';
import chalk from 'chalk';
export const prompt = promptSync({ sigint: true });
const actionErrorHandler = (error) => {
    console.error(chalk.red(error.message));
    process.exit(1);
};
export const actionRunner = (fn) => {
    return async (...args) => await fn(...args).catch(actionErrorHandler);
};
export const serializeError = (e) => {
    if (typeof e === 'string')
        return e;
    return JSON.stringify(e);
};
