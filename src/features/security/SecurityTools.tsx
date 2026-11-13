import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Notice } from "@/components/ui/Display";
import { Field, PasswordInput, Select, Switch } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";

export function passwordProblem(password: string, confirm: string): string | null {
  if (!password) return "Enter a password.";
  if (/[\r\n]/.test(password)) return "Passwords cannot contain line breaks.";
  if (password !== confirm) return "The passwords do not match.";
  return null;
}

export function ProtectTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [owner, setOwner] = useState("");
  const [encryption, setEncryption] = useState<"aes256" | "aes128">("aes256");
  const [printing, setPrinting] = useState<"full" | "low" | "none">("full");
  const [copying, setCopying] = useState(true);
  const [modifying, setModifying] = useState(true);
  const [annotating, setAnnotating] = useState(true);
  const [forms, setForms] = useState(true);
  const problem = passwordProblem(password, confirm);

  return (
    <ToolWorkspace
      tool={getTool("protect")!}
      selection={selection}
      blocker={problem}
      buildOptions={() => ({
        userPassword: password,
        ownerPassword: owner,
        encryption,
        printing,
        allowCopying: copying,
        allowModifying: modifying,
        allowAnnotating: annotating,
        allowFormFilling: forms,
      })}
      options={
        <>
          <Field label="Password to open" error={confirm && password !== confirm ? "The passwords do not match." : null}>
            {(id, d) => <PasswordInput id={id} aria-describedby={d} value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="new-password" />}
          </Field>
          <Field label="Confirm password">
            {(id) => <PasswordInput id={id} value={confirm} onChange={(e) => setConfirm(e.target.value)} autoComplete="new-password" />}
          </Field>
          <Field label="Owner password (optional)" hint="Needed to change permissions later. Leave empty to generate a random one.">
            {(id, d) => <PasswordInput id={id} aria-describedby={d} value={owner} onChange={(e) => setOwner(e.target.value)} autoComplete="new-password" />}
          </Field>
          <Field label="Encryption">
            {(id) => (
              <Select id={id} value={encryption} onChange={(e) => setEncryption(e.target.value as typeof encryption)}>
                <option value="aes256">AES 256-bit (recommended)</option>
                <option value="aes128">AES 128-bit (older readers)</option>
              </Select>
            )}
          </Field>
          <div className="flex flex-col gap-3">
            <span className="text-secondary font-medium">Permissions</span>
            <Field label="Printing">
              {(id) => (
                <Select id={id} value={printing} onChange={(e) => setPrinting(e.target.value as typeof printing)}>
                  <option value="full">Allowed</option>
                  <option value="low">Low resolution only</option>
                  <option value="none">Not allowed</option>
                </Select>
              )}
            </Field>
            <Switch checked={copying} onChange={setCopying} label="Allow copying text and images" />
            <Switch checked={modifying} onChange={setModifying} label="Allow editing" />
            <Switch checked={annotating} onChange={setAnnotating} label="Allow comments" />
            <Switch checked={forms} onChange={setForms} label="Allow filling forms" />
          </div>
          <Notice tone="accent" icon={ShieldCheck}>
            Passwords are used once and never stored. Keep them safe: they cannot be recovered.
          </Notice>
        </>
      }
    />
  );
}

export function UnlockTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [password, setPassword] = useState("");
  const unprotected = selection.files.filter((f) => !f.encrypted);
  const blocker = !password ? "Enter the document's password." : unprotected.length ? `“${unprotected[0].name}” is not password-protected.` : null;

  return (
    <ToolWorkspace
      tool={getTool("unlock")!}
      selection={selection}
      blocker={blocker}
      buildOptions={() => ({ password })}
      options={
        <>
          <Field label="Password" hint="The password used to open the document, or its owner password.">
            {(id, d) => <PasswordInput id={id} aria-describedby={d} value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="off" />}
          </Field>
          <p className="text-caption text-muted">
            PaperFlow removes protection only with the correct password. It cannot recover or guess forgotten passwords.
          </p>
        </>
      }
    />
  );
}
