import { useEffect, useState } from "react";
import { api, errorMessage } from "@/lib/api";
import { useSession } from "@/state/session";
import { useToast } from "@/state/toast";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { FieldLabel, Textarea } from "@/components/ui/Field";
import { InlineError } from "@/components/ui/Feedback";

export type ModerationAction = "kick" | "ban" | "unban";

const COPY: Record<ModerationAction, { title: string; blurb: string; label: string; destructive: boolean }> = {
  kick: {
    title: "Kick player",
    blurb:
      "Runs the :kick server command through the ER:LC API. The player is removed from the server; the action is recorded in CAD.",
    label: "Kick player",
    destructive: true,
  },
  ban: {
    title: "Ban player",
    blurb:
      "Runs the :ban server command through the ER:LC API. This bans the player on your ER:LC server, not just in CAD.",
    label: "Ban player",
    destructive: true,
  },
  unban: {
    title: "Unban player",
    blurb:
      "Runs the :unban server command through the ER:LC API to lift a server ban. Requires the moderation.unban permission.",
    label: "Unban player",
    destructive: true,
  },
};

export function ModerationActionDialog({
  open,
  action,
  target,
  robloxUserId,
  onClose,
  onDone,
}: {
  open: boolean;
  action: ModerationAction;
  target: string;
  robloxUserId: number | null;
  onClose: () => void;
  onDone?: () => void;
}) {
  const { token } = useSession();
  const toast = useToast();
  const [reason, setReason] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const copy = COPY[action];

  useEffect(() => {
    if (open) {
      setReason("");
      setError(null);
    }
  }, [open, action]);

  async function submit() {
    if (!token) return;
    setSubmitting(true);
    setError(null);
    try {
      const call =
        action === "kick"
          ? api.kickPlayer(token, target, reason, true, robloxUserId)
          : action === "ban"
            ? api.banPlayer(token, target, reason, true, robloxUserId)
            : api.unbanPlayer(token, target, reason, true, robloxUserId);
      const result = await call;
      toast.success(`${copy.label} sent`, result.message);
      onDone?.();
      onClose();
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      toast.error(`${copy.label} failed`, message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={`${copy.title} — ${target}`}
      description={copy.blurb}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={submitting}>
            Cancel
          </Button>
          <Button variant="danger" onClick={submit} loading={submitting}>
            {copy.label}
          </Button>
        </>
      }
    >
      <div className="space-y-3">
        <div className="rounded-md border border-line bg-surface-2 px-3 py-2.5">
          <p className="text-xs leading-5 text-muted">
            This sends a real command to your ER:LC server as{" "}
            <span className="text-ink">virtual server management</span>. It cannot be undone from
            CAD and is never retried automatically.
          </p>
        </div>
        <div>
          <FieldLabel htmlFor="moderation-reason">Reason (stored in CAD)</FieldLabel>
          <Textarea
            id="moderation-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder="Briefly explain the reason for this action"
            rows={3}
          />
        </div>
        {error ? <InlineError message={error} /> : null}
      </div>
    </Modal>
  );
}
