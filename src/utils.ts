import promptSync from "prompt-sync";

export const prompt = promptSync({ sigint: true });

const actionErrorHandler = (error: Error): void => {
  console.error(error.message);
  process.exit(1);
};

export const actionRunner = (fn: (...args) => Promise<any>) => {
  return async (...args) => await fn(...args).catch(actionErrorHandler);
};
