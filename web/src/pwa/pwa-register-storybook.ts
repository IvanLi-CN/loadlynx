type RegisterState<T> = [T, (value: T) => void];

export function useRegisterSW() {
  const state = <T>(value: T): RegisterState<T> => [value, () => {}];

  return {
    offlineReady: state(false),
    needRefresh: state(false),
    updateServiceWorker: async () => {},
  };
}
