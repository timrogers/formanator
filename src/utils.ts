import promptSync from 'prompt-sync';
import chalk from 'chalk';

export const prompt = promptSync({ sigint: true });

const actionErrorHandler = (error: Error): void => {
  console.error(chalk.red(error.message));
  process.exit(1);
};

export const actionRunner = (fn: (...args) => Promise<unknown>) => {
  return async (...args) => await fn(...args).catch(actionErrorHandler);
};

export const serializeError = (e: unknown): string => {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
};

export const validateAxiosStatus = (_status: number): boolean => true;
