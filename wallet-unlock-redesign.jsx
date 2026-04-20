import { useState, useEffect, useCallback } from "react";

// ─── Wallet Unlock UX Redesign ─────────────────────────────────────────────
// Core: Lock/Unlock state machine with session-based auth
// Flow: Locked → Password Modal → Unlocked (5min session) → Auto-lock
// ───────────────────────────────────────────────────────────────────────────

const C = {
  bg: "#0f1019", card: "#1a1b26", cardHover: "#1e1f2e",
  border: "#2a2b3d", borderLight: "#363752",
  accent: "#6366f1", accentGlow: "rgba(99,102,241,0.15)",
  text: "#e4e4e7", textMuted: "#71717a", textDim: "#52525b",
  danger: "#ef4444", dangerBg: "rgba(239,68,68,0.08)",
  success: "#22c55e", successBg: "rgba(34,197,94,0.08)",
  warning: "#f59e0b", warningBg: "rgba(245,158,11,0.08)",
  orange: "#f97316", orangeBg: "rgba(249,115,22,0.08)",
};

// ─── SVG Icons ──────────────────────────────────────────────────────────────
const Icons = {
  lock: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0110 0v4"/></svg>,
  unlock: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 019.9-1"/></svg>,
  shield: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>,
  shieldCheck: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M9 12l2 2 4-4"/></svg>,
  eye: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>,
  eyeOff: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>,
  copy: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>,
  check: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12"/></svg>,
  send: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>,
  key: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 11-7.778 7.778 5.5 5.5 0 017.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg>,
  x: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>,
  refresh: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>,
  trash: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>,
  more: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="currentColor"><circle cx="12" cy="5" r="1.5"/><circle cx="12" cy="12" r="1.5"/><circle cx="12" cy="19" r="1.5"/></svg>,
  warn: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>,
  timer: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>,
  lockOpen: (s=16) => <svg width={s} height={s} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0110 0v4"/><circle cx="12" cy="16" r="1"/></svg>,
};

// ─── Animations CSS ─────────────────────────────────────────────────────────
const STYLES = `
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideUp { from { opacity: 0; transform: translateY(16px) scale(0.97); } to { opacity: 1; transform: translateY(0) scale(1); } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
  @keyframes shake { 0%, 100% { transform: translateX(0); } 10%, 30%, 50%, 70%, 90% { transform: translateX(-4px); } 20%, 40%, 60%, 80% { transform: translateX(4px); } }
  @keyframes lockBounce { 0% { transform: scale(1); } 30% { transform: scale(1.2); } 50% { transform: scale(0.9); } 70% { transform: scale(1.05); } 100% { transform: scale(1); } }
  @keyframes unlockSlide { 0% { transform: translateY(0) rotate(0); } 50% { transform: translateY(-3px) rotate(-10deg); } 100% { transform: translateY(0) rotate(0); } }
  @keyframes progressShrink { from { width: 100%; } to { width: 0%; } }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  input::placeholder { color: ${C.textDim}; }
  button { font-family: inherit; }
`;

// ─── Lock Status Badge ──────────────────────────────────────────────────────
const LockStatusBadge = ({ isUnlocked, remainingSeconds, onLock }) => {
  const mm = Math.floor(remainingSeconds / 60);
  const ss = String(remainingSeconds % 60).padStart(2, "0");

  if (!isUnlocked) {
    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 8,
        padding: "8px 16px", borderRadius: 12,
        background: C.orangeBg, border: "1px solid rgba(249,115,22,0.2)",
      }}>
        <span style={{ color: C.orange, display: "flex" }}>{Icons.lock(16)}</span>
        <span style={{ color: C.orange, fontSize: 13, fontWeight: 600 }}>Locked</span>
      </div>
    );
  }

  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 8,
      padding: "8px 16px", borderRadius: 12,
      background: C.successBg, border: "1px solid rgba(34,197,94,0.2)",
      position: "relative", overflow: "hidden",
    }}>
      {/* Session countdown bar */}
      <div style={{
        position: "absolute", bottom: 0, left: 0, height: 2,
        background: C.success, opacity: 0.4,
        animation: `progressShrink ${remainingSeconds}s linear forwards`,
      }} />
      <span style={{ color: C.success, display: "flex", animation: "unlockSlide 0.5s ease-out" }}>
        {Icons.unlock(16)}
      </span>
      <span style={{ color: C.success, fontSize: 13, fontWeight: 600 }}>
        Unlocked
      </span>
      <span style={{ color: C.success, fontSize: 12, fontFamily: "monospace", opacity: 0.8 }}>
        {mm}:{ss}
      </span>
      <button onClick={onLock} title="Lock now" style={{
        background: "none", border: "none", cursor: "pointer",
        color: "rgba(34,197,94,0.5)", display: "flex", padding: 2,
        marginLeft: 2, borderRadius: 4, transition: "all 0.15s",
      }}
        onMouseEnter={e => e.currentTarget.style.color = C.success}
        onMouseLeave={e => e.currentTarget.style.color = "rgba(34,197,94,0.5)"}
      >
        {Icons.lock(14)}
      </button>
    </div>
  );
};

// ─── Password Unlock Modal ──────────────────────────────────────────────────
const PasswordModal = ({ isOpen, onClose, onConfirm, title, description }) => {
  const [pw, setPw] = useState("");
  const [showPw, setShowPw] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [shaking, setShaking] = useState(false);

  useEffect(() => {
    if (isOpen) { setPw(""); setError(""); setShowPw(false); setLoading(false); setShaking(false); }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = async () => {
    if (!pw) { setError("Password is required"); return; }
    setLoading(true); setError("");
    await new Promise(r => setTimeout(r, 800));
    // Demo: password must be 3+ chars
    if (pw.length < 3) {
      setError("Incorrect password");
      setShaking(true);
      setTimeout(() => setShaking(false), 500);
      setLoading(false);
      return;
    }
    onConfirm(pw);
    setLoading(false);
  };

  return (
    <div style={{
      position: "fixed", inset: 0, zIndex: 1000,
      background: "rgba(0,0,0,0.65)", backdropFilter: "blur(12px)",
      display: "flex", alignItems: "center", justifyContent: "center",
      animation: "fadeIn 0.15s ease-out",
    }} onClick={onClose}>
      <div style={{
        background: C.card, borderRadius: 24, width: 420, maxWidth: "92vw",
        border: `1px solid ${C.border}`,
        boxShadow: "0 32px 64px -16px rgba(0,0,0,0.6)",
        animation: shaking ? "shake 0.4s ease-out" : "slideUp 0.25s ease-out",
      }} onClick={e => e.stopPropagation()}>

        {/* Lock Icon Header */}
        <div style={{ padding: "32px 32px 0", textAlign: "center" }}>
          <div style={{
            width: 72, height: 72, borderRadius: 20, margin: "0 auto 20px",
            background: `linear-gradient(135deg, ${C.accentGlow}, rgba(139,92,246,0.12))`,
            display: "flex", alignItems: "center", justifyContent: "center",
            border: "1px solid rgba(99,102,241,0.25)",
            animation: "lockBounce 0.5s ease-out",
          }}>
            <span style={{ color: C.accent }}>{Icons.lockOpen(36)}</span>
          </div>
          <h2 style={{ color: C.text, fontSize: 20, fontWeight: 700, margin: "0 0 8px" }}>{title}</h2>
          <p style={{ color: C.textMuted, fontSize: 13, lineHeight: 1.6, margin: 0, maxWidth: 300, marginLeft: "auto", marginRight: "auto" }}>{description}</p>
        </div>

        {/* Password Input */}
        <div style={{ padding: "28px 32px 32px" }}>
          <label style={{ display: "block", fontSize: 11, fontWeight: 700, color: C.textMuted, marginBottom: 10, textTransform: "uppercase", letterSpacing: "0.08em" }}>
            Password
          </label>
          <div style={{ position: "relative" }}>
            <input
              type={showPw ? "text" : "password"}
              value={pw}
              onChange={e => { setPw(e.target.value); setError(""); }}
              onKeyDown={e => e.key === "Enter" && handleSubmit()}
              placeholder="Enter your wallet password"
              autoFocus
              style={{
                width: "100%", height: 52, padding: "0 48px 0 18",
                background: C.bg, border: `2px solid ${error ? C.danger : pw ? C.accent : C.border}`,
                borderRadius: 14, color: C.text, fontSize: 15,
                outline: "none", boxSizing: "border-box",
                transition: "border-color 0.2s, box-shadow 0.2s",
                boxShadow: pw && !error ? `0 0 0 3px ${C.accentGlow}` : error ? "0 0 0 3px rgba(239,68,68,0.1)" : "none",
              }}
            />
            <button onClick={() => setShowPw(!showPw)} style={{
              position: "absolute", right: 14, top: "50%", transform: "translateY(-50%)",
              background: "none", border: "none", cursor: "pointer", color: C.textMuted,
              padding: 4, display: "flex", borderRadius: 6,
            }}>
              {showPw ? Icons.eyeOff(18) : Icons.eye(18)}
            </button>
          </div>

          {/* Error Message */}
          {error && (
            <div style={{
              display: "flex", alignItems: "center", gap: 6,
              marginTop: 10, padding: "8px 12px", borderRadius: 10,
              background: C.dangerBg, border: "1px solid rgba(239,68,68,0.15)",
            }}>
              <span style={{ color: C.danger, display: "flex", flexShrink: 0 }}>{Icons.warn(14)}</span>
              <span style={{ color: "#fca5a5", fontSize: 12, fontWeight: 500 }}>{error}</span>
            </div>
          )}

          {/* Session note */}
          <div style={{
            display: "flex", alignItems: "center", gap: 6,
            marginTop: 16, padding: "8px 12px", borderRadius: 10,
            background: "rgba(99,102,241,0.05)", border: "1px solid rgba(99,102,241,0.1)",
          }}>
            <span style={{ color: C.accent, display: "flex", flexShrink: 0 }}>{Icons.timer(13)}</span>
            <span style={{ color: C.textMuted, fontSize: 11 }}>
              Once unlocked, your session stays active for 5 minutes
            </span>
          </div>

          {/* Buttons */}
          <div style={{ display: "flex", gap: 12, marginTop: 24 }}>
            <button onClick={onClose} style={{
              flex: 1, height: 50, borderRadius: 14, border: `1px solid ${C.border}`,
              background: "transparent", color: C.textMuted, fontSize: 14, fontWeight: 600,
              cursor: "pointer", transition: "all 0.15s",
            }}
              onMouseEnter={e => { e.target.style.background = C.cardHover; e.target.style.color = C.text; }}
              onMouseLeave={e => { e.target.style.background = "transparent"; e.target.style.color = C.textMuted; }}
            >Cancel</button>
            <button onClick={handleSubmit} disabled={loading || !pw} style={{
              flex: 1.2, height: 50, borderRadius: 14, border: "none",
              background: loading || !pw ? C.textDim : `linear-gradient(135deg, ${C.accent}, #8b5cf6)`,
              color: "#fff", fontSize: 14, fontWeight: 700,
              cursor: loading || !pw ? "not-allowed" : "pointer",
              display: "flex", alignItems: "center", justifyContent: "center", gap: 8,
              opacity: loading || !pw ? 0.5 : 1,
              transition: "all 0.2s",
              boxShadow: loading || !pw ? "none" : "0 4px 16px rgba(99,102,241,0.3)",
            }}>
              {loading ? (
                <div style={{ width: 20, height: 20, border: "2px solid rgba(255,255,255,0.3)", borderTopColor: "#fff", borderRadius: "50%", animation: "spin 0.6s linear infinite" }} />
              ) : (
                <>{Icons.unlock(18)} Unlock</>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

// ─── Secret Reveal Modal ────────────────────────────────────────────────────
const SecretRevealModal = ({ isOpen, onClose, secretType, secret }) => {
  const [copied, setCopied] = useState(false);
  const [revealed, setRevealed] = useState(false);
  useEffect(() => { if (isOpen) { setCopied(false); setRevealed(false); } }, [isOpen]);
  if (!isOpen) return null;

  const words = secretType === "mnemonic" ? secret.split(" ") : null;
  const handleCopy = () => { setCopied(true); setTimeout(() => setCopied(false), 2000); };

  return (
    <div style={{
      position: "fixed", inset: 0, zIndex: 1000,
      background: "rgba(0,0,0,0.65)", backdropFilter: "blur(12px)",
      display: "flex", alignItems: "center", justifyContent: "center",
    }} onClick={onClose}>
      <div style={{
        background: C.card, borderRadius: 24, width: 500, maxWidth: "92vw",
        border: `1px solid ${C.border}`,
        boxShadow: "0 32px 64px -16px rgba(0,0,0,0.6)",
        animation: "slideUp 0.25s ease-out",
      }} onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", padding: "20px 24px", borderBottom: `1px solid ${C.border}` }}>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ color: C.warning }}>{Icons.key(20)}</span>
            <h2 style={{ color: C.text, fontSize: 16, fontWeight: 700, margin: 0 }}>
              {secretType === "mnemonic" ? "Secret Recovery Phrase" : "Private Key"}
            </h2>
          </div>
          <button onClick={onClose} style={{ background: "none", border: "none", color: C.textMuted, cursor: "pointer", padding: 4, display: "flex", borderRadius: 8 }}>
            {Icons.x(20)}
          </button>
        </div>

        {/* Warning */}
        <div style={{ padding: "16px 24px 0" }}>
          <div style={{ background: C.dangerBg, border: "1px solid rgba(239,68,68,0.15)", borderRadius: 12, padding: "12px 16px", display: "flex", gap: 12, alignItems: "flex-start" }}>
            <span style={{ color: C.danger, flexShrink: 0, marginTop: 1 }}>{Icons.warn(16)}</span>
            <p style={{ color: "#fca5a5", fontSize: 12, lineHeight: 1.6, margin: 0 }}>
              Never share this with anyone. Anyone with this {secretType === "mnemonic" ? "phrase" : "key"} can take your assets permanently.
            </p>
          </div>
        </div>

        {/* Secret Content */}
        <div style={{ padding: "20px 24px" }}>
          {!revealed ? (
            <div onClick={() => setRevealed(true)} style={{
              background: C.bg, borderRadius: 16, padding: "48px 24px",
              display: "flex", flexDirection: "column", alignItems: "center", gap: 16,
              border: `2px dashed ${C.border}`, cursor: "pointer", transition: "all 0.2s",
            }}
              onMouseEnter={e => { e.currentTarget.style.borderColor = C.accent; e.currentTarget.style.background = "rgba(99,102,241,0.03)"; }}
              onMouseLeave={e => { e.currentTarget.style.borderColor = C.border; e.currentTarget.style.background = C.bg; }}
            >
              <div style={{
                width: 56, height: 56, borderRadius: 16,
                background: C.accentGlow, display: "flex", alignItems: "center", justifyContent: "center",
                border: "1px solid rgba(99,102,241,0.25)",
              }}>
                <span style={{ color: C.accent }}>{Icons.eye(28)}</span>
              </div>
              <div style={{ textAlign: "center" }}>
                <p style={{ color: C.text, fontSize: 15, fontWeight: 600, margin: "0 0 4px" }}>Click to reveal</p>
                <p style={{ color: C.textMuted, fontSize: 12, margin: 0 }}>Make sure no one else can see your screen</p>
              </div>
            </div>
          ) : words ? (
            <div style={{ background: C.bg, borderRadius: 16, padding: 20, border: `1px solid ${C.border}` }}>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 8 }}>
                {words.map((w, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, background: C.cardHover, borderRadius: 10, padding: "9px 12px" }}>
                    <span style={{ color: C.textDim, fontSize: 11, fontFamily: "monospace", minWidth: 22 }}>{i + 1}.</span>
                    <span style={{ color: C.text, fontSize: 13, fontFamily: "monospace", fontWeight: 600 }}>{w}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div style={{ background: C.bg, borderRadius: 16, padding: 20, border: `1px solid ${C.border}` }}>
              <p style={{ color: C.text, fontSize: 13, fontFamily: "monospace", wordBreak: "break-all", lineHeight: 1.8, margin: 0 }}>{secret}</p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div style={{ padding: "0 24px 24px", display: "flex", gap: 12 }}>
          {revealed && (
            <button onClick={handleCopy} style={{
              flex: 1, height: 46, borderRadius: 14, border: `1px solid ${C.border}`,
              background: "transparent", color: C.text, fontSize: 13, fontWeight: 600,
              cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", gap: 8,
              transition: "all 0.15s",
            }}
              onMouseEnter={e => e.target.style.background = C.cardHover}
              onMouseLeave={e => e.target.style.background = "transparent"}
            >
              {copied ? Icons.check(16) : Icons.copy(16)} {copied ? "Copied!" : "Copy"}
            </button>
          )}
          <button onClick={onClose} style={{
            flex: 1, height: 46, borderRadius: 14, border: "none",
            background: `linear-gradient(135deg, ${C.accent}, #8b5cf6)`, color: "#fff", fontSize: 13, fontWeight: 700,
            cursor: "pointer",
          }}>Done</button>
        </div>
      </div>
    </div>
  );
};

// ─── More Menu ──────────────────────────────────────────────────────────────
const MoreMenu = ({ wallet, onAction, isUnlocked }) => {
  const [open, setOpen] = useState(false);
  const items = [
    { id: "private-key", icon: "key", label: "Export Private Key", sub: isUnlocked ? "Session unlocked" : "Requires password", color: C.text, subColor: isUnlocked ? C.success : C.textDim },
    ...(wallet.type === "mnemonic" ? [{ id: "mnemonic", icon: "key", label: "Export Recovery Phrase", sub: isUnlocked ? "Session unlocked" : "Requires password", color: C.text, subColor: isUnlocked ? C.success : C.textDim }] : []),
    { id: "divider" },
    { id: "delete", icon: "trash", label: "Remove Wallet", sub: "This cannot be undone", color: C.danger, subColor: "rgba(239,68,68,0.5)" },
  ];

  return (
    <div style={{ position: "relative" }}>
      <button onClick={() => setOpen(!open)} style={{
        width: 38, height: 38, borderRadius: 12,
        background: open ? C.cardHover : "transparent", border: `1px solid ${open ? C.borderLight : "transparent"}`,
        color: C.textMuted, cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", transition: "all 0.15s",
      }}
        onMouseEnter={e => { if (!open) { e.currentTarget.style.background = C.cardHover; e.currentTarget.style.borderColor = C.border; } }}
        onMouseLeave={e => { if (!open) { e.currentTarget.style.background = "transparent"; e.currentTarget.style.borderColor = "transparent"; } }}
      >{Icons.more(18)}</button>

      {open && (<>
        <div style={{ position: "fixed", inset: 0, zIndex: 99 }} onClick={() => setOpen(false)} />
        <div style={{
          position: "absolute", right: 0, top: "100%", marginTop: 6, zIndex: 100,
          background: C.card, border: `1px solid ${C.border}`, borderRadius: 16,
          boxShadow: "0 24px 48px -12px rgba(0,0,0,0.5)",
          minWidth: 240, padding: 6, animation: "fadeIn 0.1s ease-out",
        }}>
          {items.map((item, i) => item.id === "divider" ? (
            <div key={i} style={{ height: 1, background: C.border, margin: "4px 10px" }} />
          ) : (
            <button key={item.id} onClick={() => { setOpen(false); onAction(item.id); }} style={{
              width: "100%", padding: "10px 14px", borderRadius: 12,
              background: "transparent", border: "none", cursor: "pointer",
              display: "flex", alignItems: "center", gap: 12,
              textAlign: "left", transition: "background 0.1s",
            }}
              onMouseEnter={e => e.currentTarget.style.background = item.id === "delete" ? C.dangerBg : C.cardHover}
              onMouseLeave={e => e.currentTarget.style.background = "transparent"}
            >
              <div style={{
                width: 32, height: 32, borderRadius: 10, flexShrink: 0,
                background: item.id === "delete" ? C.dangerBg : C.accentGlow,
                display: "flex", alignItems: "center", justifyContent: "center",
                color: item.color,
              }}>{item.icon === "key" ? Icons.key(16) : Icons.trash(16)}</div>
              <div>
                <div style={{ color: item.color, fontSize: 13, fontWeight: 600 }}>{item.label}</div>
                <div style={{ color: item.subColor, fontSize: 11, marginTop: 1, display: "flex", alignItems: "center", gap: 4 }}>
                  {item.subColor === C.success && Icons.shieldCheck(11)}
                  {item.sub}
                </div>
              </div>
            </button>
          ))}
        </div>
      </>)}
    </div>
  );
};

// ─── Delete Confirm Modal ───────────────────────────────────────────────────
const DeleteModal = ({ isOpen, onClose, onConfirm, walletLabel }) => {
  const [text, setText] = useState("");
  useEffect(() => { if (isOpen) setText(""); }, [isOpen]);
  if (!isOpen) return null;
  const canDelete = text.toLowerCase() === "remove";

  return (
    <div style={{ position: "fixed", inset: 0, zIndex: 1000, background: "rgba(0,0,0,0.65)", backdropFilter: "blur(12px)", display: "flex", alignItems: "center", justifyContent: "center" }} onClick={onClose}>
      <div style={{ background: C.card, borderRadius: 24, width: 420, maxWidth: "92vw", border: `1px solid ${C.border}`, boxShadow: "0 32px 64px -16px rgba(0,0,0,0.6)", animation: "slideUp 0.25s ease-out" }} onClick={e => e.stopPropagation()}>
        <div style={{ padding: "32px 32px 0", textAlign: "center" }}>
          <div style={{ width: 72, height: 72, borderRadius: 20, margin: "0 auto 20px", background: C.dangerBg, display: "flex", alignItems: "center", justifyContent: "center", border: "1px solid rgba(239,68,68,0.2)" }}>
            <span style={{ color: C.danger }}>{Icons.trash(36)}</span>
          </div>
          <h2 style={{ color: C.text, fontSize: 20, fontWeight: 700, margin: "0 0 8px" }}>Remove Wallet</h2>
          <p style={{ color: C.textMuted, fontSize: 13, lineHeight: 1.6, margin: 0 }}>This will remove <strong style={{ color: C.text }}>{walletLabel}</strong> from this device. Make sure you've backed up your keys.</p>
        </div>
        <div style={{ padding: "24px 32px 32px" }}>
          <label style={{ fontSize: 11, fontWeight: 700, color: C.textMuted, textTransform: "uppercase", letterSpacing: "0.08em", display: "block", marginBottom: 10 }}>Type "remove" to confirm</label>
          <input value={text} onChange={e => setText(e.target.value)} placeholder="remove" autoFocus style={{
            width: "100%", height: 48, padding: "0 18px", background: C.bg, border: `2px solid ${canDelete ? C.danger : C.border}`, borderRadius: 14, color: C.text, fontSize: 15, outline: "none", boxSizing: "border-box", transition: "border-color 0.2s",
          }} />
          <div style={{ display: "flex", gap: 12, marginTop: 24 }}>
            <button onClick={onClose} style={{ flex: 1, height: 50, borderRadius: 14, border: `1px solid ${C.border}`, background: "transparent", color: C.textMuted, fontSize: 14, fontWeight: 600, cursor: "pointer" }}>Cancel</button>
            <button onClick={onConfirm} disabled={!canDelete} style={{
              flex: 1, height: 50, borderRadius: 14, border: "none",
              background: canDelete ? C.danger : C.textDim, color: "#fff", fontSize: 14, fontWeight: 700,
              cursor: canDelete ? "pointer" : "not-allowed", opacity: canDelete ? 1 : 0.4,
            }}>Remove</button>
          </div>
        </div>
      </div>
    </div>
  );
};

// ─── Wallet Card ────────────────────────────────────────────────────────────
const WalletCard = ({ wallet, isUnlocked, onAction }) => {
  const [expanded, setExpanded] = useState(wallet.chains.some(c => c.totalUsd > 0));
  const [addrCopied, setAddrCopied] = useState(false);

  const copyAddr = () => { setAddrCopied(true); setTimeout(() => setAddrCopied(false), 1500); };

  return (
    <div style={{
      background: C.card, borderRadius: 20, border: `1px solid ${C.border}`,
      overflow: "hidden", transition: "border-color 0.2s",
    }}
      onMouseEnter={e => e.currentTarget.style.borderColor = C.borderLight}
      onMouseLeave={e => e.currentTarget.style.borderColor = C.border}
    >
      {/* Header */}
      <div style={{ padding: "20px 22px", display: "flex", alignItems: "center", gap: 14 }}>
        {/* Avatar */}
        <div style={{
          width: 48, height: 48, borderRadius: 14, flexShrink: 0,
          background: "linear-gradient(135deg, #6366f1, #8b5cf6)",
          display: "flex", alignItems: "center", justifyContent: "center",
          boxShadow: "0 6px 16px rgba(99,102,241,0.3)",
          fontSize: 20, color: "#fff", fontWeight: 700,
        }}>
          {Icons.shield(24)}
        </div>

        {/* Name + Address */}
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ color: C.text, fontSize: 16, fontWeight: 700 }}>{wallet.label}</span>
            <span style={{
              fontSize: 10, fontWeight: 700, padding: "3px 9px", borderRadius: 7,
              background: C.accentGlow, color: C.accent, textTransform: "uppercase",
            }}>{wallet.type === "mnemonic" ? "HD" : "Imported"}</span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 5 }}>
            <span style={{ color: C.textMuted, fontSize: 13, fontFamily: "monospace" }}>{wallet.address}</span>
            <button onClick={copyAddr} style={{
              background: "none", border: "none", color: addrCopied ? C.success : C.textDim,
              cursor: "pointer", padding: 2, display: "flex", transition: "color 0.15s",
            }}>
              {addrCopied ? Icons.check(13) : Icons.copy(13)}
            </button>
          </div>
        </div>

        {/* Balance */}
        <div style={{ textAlign: "right", marginRight: 8 }}>
          <p style={{ color: C.text, fontSize: 22, fontWeight: 800, fontFamily: "monospace", margin: 0 }}>
            ${wallet.balance.toLocaleString("en-US", { minimumFractionDigits: 2 })}
          </p>
          <p style={{ color: C.textMuted, fontSize: 11, margin: "3px 0 0" }}>Updated {wallet.updated}</p>
        </div>

        {/* Action Buttons */}
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {/* Send */}
          <button style={{
            width: 38, height: 38, borderRadius: 12,
            background: C.accentGlow, border: "1px solid rgba(99,102,241,0.2)",
            color: C.accent, cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center",
            transition: "all 0.15s",
          }}
            onMouseEnter={e => { e.currentTarget.style.background = C.accent; e.currentTarget.style.color = "#fff"; }}
            onMouseLeave={e => { e.currentTarget.style.background = C.accentGlow; e.currentTarget.style.color = C.accent; }}
            title="Send"
          >{Icons.send(18)}</button>
          {/* Refresh */}
          <button style={{
            width: 38, height: 38, borderRadius: 12,
            background: "transparent", border: "1px solid transparent",
            color: C.textMuted, cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center",
            transition: "all 0.15s",
          }}
            onMouseEnter={e => { e.currentTarget.style.background = C.cardHover; e.currentTarget.style.borderColor = C.border; }}
            onMouseLeave={e => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.borderColor = "transparent"; }}
            title="Refresh"
          >{Icons.refresh(17)}</button>
          {/* More */}
          <MoreMenu wallet={wallet} isUnlocked={isUnlocked} onAction={(action) => onAction(action, wallet)} />
        </div>
      </div>

      {/* Assets */}
      {wallet.chains.length > 0 && (
        <div style={{ borderTop: `1px solid ${C.border}` }}>
          <button onClick={() => setExpanded(!expanded)} style={{
            width: "100%", padding: "11px 22px",
            background: "transparent", border: "none", cursor: "pointer",
            display: "flex", alignItems: "center", justifyContent: "space-between",
            color: C.textMuted, fontSize: 11, fontWeight: 700,
            textTransform: "uppercase", letterSpacing: "0.06em",
          }}>
            <span>Assets by chain</span>
            <span style={{ transform: expanded ? "rotate(180deg)" : "rotate(0deg)", transition: "transform 0.2s", fontSize: 14 }}>▾</span>
          </button>

          {expanded && wallet.chains.map((chain, ci) => (
            <div key={ci}>
              <div style={{ padding: "10px 22px", display: "flex", alignItems: "center", justifyContent: "space-between", background: "rgba(0,0,0,0.2)", borderTop: `1px solid ${C.border}` }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ color: C.text, fontSize: 13, fontWeight: 700 }}>{chain.name}</span>
                  <span style={{ color: C.textDim, fontSize: 11 }}>ID: {chain.chainId}</span>
                </div>
                <span style={{ color: C.text, fontSize: 13, fontFamily: "monospace", fontWeight: 700 }}>${chain.totalUsd.toFixed(2)}</span>
              </div>
              {chain.assets.map((a, ai) => (
                <div key={ai} style={{
                  padding: "10px 22px 10px 38px", display: "flex", alignItems: "center", justifyContent: "space-between",
                  borderTop: `1px solid rgba(42,43,61,0.5)`, transition: "background 0.1s",
                }}
                  onMouseEnter={e => e.currentTarget.style.background = "rgba(255,255,255,0.015)"}
                  onMouseLeave={e => e.currentTarget.style.background = "transparent"}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                    <div style={{ width: 30, height: 30, borderRadius: 9, background: C.cardHover, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 12, fontWeight: 700, color: C.textMuted }}>{a.symbol[0]}</div>
                    <div>
                      <span style={{ color: C.text, fontSize: 13, fontWeight: 600 }}>{a.symbol}</span>
                      <span style={{ color: C.textDim, fontSize: 11, marginLeft: 6 }}>{a.name}</span>
                    </div>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
                    <div style={{ textAlign: "right" }}>
                      <p style={{ color: C.text, fontSize: 12, fontFamily: "monospace", margin: 0 }}>{a.balance} {a.symbol}</p>
                      {a.usdValue > 0 && <p style={{ color: C.textMuted, fontSize: 11, fontFamily: "monospace", margin: "2px 0 0" }}>${a.usdValue.toFixed(2)}</p>}
                    </div>
                    <button style={{
                      width: 28, height: 28, borderRadius: 8, background: "transparent", border: "none",
                      color: C.textDim, cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center", transition: "all 0.15s",
                    }}
                      onMouseEnter={e => { e.currentTarget.style.color = C.accent; e.currentTarget.style.background = C.accentGlow; }}
                      onMouseLeave={e => { e.currentTarget.style.color = C.textDim; e.currentTarget.style.background = "transparent"; }}
                      title={`Send ${a.symbol}`}
                    >{Icons.send(14)}</button>
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

// ─── Flow Diagram ───────────────────────────────────────────────────────────
const FlowDiagram = () => {
  const steps = [
    { icon: "more", label: "Click ⋮ Menu", sub: "Select sensitive action", color: C.textMuted, bg: C.cardHover },
    { icon: "lock", label: "Password Modal", sub: "Verify identity", color: C.orange, bg: C.orangeBg },
    { icon: "unlock", label: "Session Unlocked", sub: "5 min active window", color: C.success, bg: C.successBg },
    { icon: "eye", label: "Reveal Secret", sub: "Click to view", color: C.accent, bg: C.accentGlow },
  ];

  return (
    <div style={{
      background: C.card, borderRadius: 20, padding: 24, marginBottom: 16,
      border: `1px solid ${C.border}`,
    }}>
      <p style={{ color: C.textMuted, fontSize: 11, fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.08em", marginBottom: 16 }}>
        Interaction Flow
      </p>
      <div style={{ display: "flex", alignItems: "center", gap: 0 }}>
        {steps.map((s, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", flex: 1 }}>
            <div style={{ textAlign: "center", flex: 1 }}>
              <div style={{
                width: 48, height: 48, borderRadius: 14, margin: "0 auto 10px",
                background: s.bg, display: "flex", alignItems: "center", justifyContent: "center",
                border: `1px solid ${s.color}25`,
              }}>
                <span style={{ color: s.color }}>{Icons[s.icon](22)}</span>
              </div>
              <p style={{ color: C.text, fontSize: 12, fontWeight: 700, margin: 0 }}>{s.label}</p>
              <p style={{ color: C.textDim, fontSize: 10, margin: "3px 0 0" }}>{s.sub}</p>
            </div>
            {i < steps.length - 1 && (
              <div style={{ color: C.textDim, fontSize: 18, margin: "0 -4px", marginBottom: 28 }}>→</div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};

// ─── Main App ───────────────────────────────────────────────────────────────
const MOCK_WALLETS = [
  {
    id: "1", label: "Main Wallet", type: "mnemonic",
    address: "0x2b3f...8a2f", balance: 467.20, updated: "21:15",
    chains: [
      { name: "Ethereum", chainId: 1, totalUsd: 0, assets: [
        { symbol: "ETH", name: "Ethereum", balance: "0.000000", usdValue: 0 },
        { symbol: "USDC", name: "USD Coin", balance: "0.000000", usdValue: 0 },
      ]},
      { name: "Sepolia", chainId: 11155111, totalUsd: 467.20, assets: [
        { symbol: "ETH", name: "Ethereum", balance: "0.200000", usdValue: 467.20 },
        { symbol: "USDC", name: "USD Coin", balance: "0.000000", usdValue: 0 },
        { symbol: "USDT", name: "Tether USD", balance: "0.000000", usdValue: 0 },
      ]},
    ]
  },
];

const MOCK_MNEMONIC = "abandon ability able about above absent absorb abstract absurd abuse access accident";
const MOCK_PK = "0x4c0883a69102937d6231471b5dbb6204fe512961708279f23efb02eb23a0f328";

export default function App() {
  const [pwModal, setPwModal] = useState({ open: false, action: null, wallet: null, title: "", desc: "" });
  const [secretModal, setSecretModal] = useState({ open: false, type: null, secret: null });
  const [deleteModal, setDeleteModal] = useState({ open: false, wallet: null });

  // ── Session Lock State ──
  const [isUnlocked, setIsUnlocked] = useState(false);
  const [remaining, setRemaining] = useState(0);
  const SESSION_DURATION = 300; // 5 min

  useEffect(() => {
    if (!isUnlocked || remaining <= 0) {
      if (isUnlocked) setIsUnlocked(false);
      return;
    }
    const t = setInterval(() => setRemaining(p => p - 1), 1000);
    return () => clearInterval(t);
  }, [isUnlocked, remaining]);

  const lockNow = () => { setIsUnlocked(false); setRemaining(0); };

  const handleAction = (action, wallet) => {
    if (action === "delete") {
      setDeleteModal({ open: true, wallet });
      return;
    }
    // If already unlocked, skip password
    if (isUnlocked) {
      revealSecret(action);
      return;
    }
    // Show password modal
    setPwModal({
      open: true, action, wallet,
      title: action === "private-key" ? "Export Private Key" : "Export Recovery Phrase",
      desc: "Enter your password to access sensitive wallet data.",
    });
  };

  const revealSecret = (action) => {
    setSecretModal({
      open: true,
      type: action === "mnemonic" ? "mnemonic" : "private-key",
      secret: action === "mnemonic" ? MOCK_MNEMONIC : MOCK_PK,
    });
  };

  const handleUnlock = (pw) => {
    setPwModal(p => ({ ...p, open: false }));
    setIsUnlocked(true);
    setRemaining(SESSION_DURATION);
    // After unlock, proceed to action
    setTimeout(() => revealSecret(pwModal.action), 200);
  };

  return (
    <div style={{ background: C.bg, minHeight: "100vh", padding: "28px 0" }}>
      <style>{STYLES}</style>

      <div style={{ maxWidth: 740, margin: "0 auto", padding: "0 20px" }}>
        {/* Header */}
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 28 }}>
          <div>
            <h1 style={{ color: C.text, fontSize: 24, fontWeight: 800, margin: 0 }}>
              Wallet Security UX
            </h1>
            <p style={{ color: C.textMuted, fontSize: 13, margin: "4px 0 0" }}>
              Lock / Unlock interaction redesign
            </p>
          </div>
          <LockStatusBadge isUnlocked={isUnlocked} remainingSeconds={remaining} onLock={lockNow} />
        </div>

        {/* Flow Diagram */}
        <FlowDiagram />

        {/* Instructions */}
        <div style={{
          background: C.accentGlow, borderRadius: 14, padding: "14px 20px", marginBottom: 16,
          border: "1px solid rgba(99,102,241,0.15)",
          display: "flex", alignItems: "center", gap: 12,
        }}>
          <span style={{ color: C.accent, flexShrink: 0 }}>{Icons.shieldCheck(18)}</span>
          <p style={{ color: C.text, fontSize: 13, margin: 0 }}>
            <strong>Try it:</strong> Click the <strong>⋮</strong> menu → <strong>Export Private Key</strong>. Enter any 3+ character password to unlock. After unlocking, try exporting again — it skips the password (session is active). Click the <span style={{ color: C.orange }}>lock icon</span> to re-lock.
          </p>
        </div>

        {/* Wallet Cards */}
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {MOCK_WALLETS.map(w => (
            <WalletCard key={w.id} wallet={w} isUnlocked={isUnlocked} onAction={handleAction} />
          ))}
        </div>
      </div>

      {/* Modals */}
      <PasswordModal
        isOpen={pwModal.open}
        onClose={() => setPwModal(p => ({ ...p, open: false }))}
        onConfirm={handleUnlock}
        title={pwModal.title}
        description={pwModal.desc}
      />
      <SecretRevealModal
        isOpen={secretModal.open}
        onClose={() => setSecretModal({ open: false, type: null, secret: null })}
        secretType={secretModal.type}
        secret={secretModal.secret}
      />
      <DeleteModal
        isOpen={deleteModal.open}
        onClose={() => setDeleteModal({ open: false, wallet: null })}
        onConfirm={() => setDeleteModal({ open: false, wallet: null })}
        walletLabel={deleteModal.wallet?.label}
      />
    </div>
  );
}
