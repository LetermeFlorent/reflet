export interface ConfirmReq {
  message: string;
  title: string;
  confirmLabel: string;
  danger: boolean;
  resolve: (ok: boolean) => void;
}

export interface ConfirmOpts {
  title?: string;
  confirmLabel?: string;
  danger?: boolean;
}

class ConfirmController {
  req = $state<ConfirmReq | null>(null);

  ask(message: string, opts: ConfirmOpts = {}): Promise<boolean> {
    return new Promise((resolve) => {
      this.req = {
        message,
        title: opts.title ?? "Confirmer",
        confirmLabel: opts.confirmLabel ?? "Confirmer",
        danger: opts.danger ?? false,
        resolve,
      };
    });
  }

  answer(ok: boolean) {
    this.req?.resolve(ok);
    this.req = null;
  }
}

export const confirmCtl = new ConfirmController();
