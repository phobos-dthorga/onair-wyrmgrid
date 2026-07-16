export type DialogNavigation<T extends string> = {
  current: T | null;
  history: T[];
};

export function closedDialogNavigation<
  T extends string,
>(): DialogNavigation<T> {
  return { current: null, history: [] };
}

export function openDialogNavigation<T extends string>(
  surface: T,
): DialogNavigation<T> {
  return { current: surface, history: [] };
}

export function enterDialogSurface<T extends string>(
  navigation: DialogNavigation<T>,
  surface: T,
): DialogNavigation<T> {
  if (navigation.current === null || navigation.current === surface) {
    return { current: surface, history: [...navigation.history] };
  }
  return {
    current: surface,
    history: [...navigation.history, navigation.current],
  };
}

export function leaveDialogSurface<T extends string>(
  navigation: DialogNavigation<T>,
): DialogNavigation<T> {
  const history = [...navigation.history];
  return { current: history.pop() ?? null, history };
}

export function isDialogSurface<T extends string>(
  navigation: DialogNavigation<T>,
  surface: T,
): boolean {
  return navigation.current === surface;
}
