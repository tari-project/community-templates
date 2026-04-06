import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "../../api/client";

export default function Login() {
  const navigate = useNavigate();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    try {
      const { token } = await api.login(username, password);
      localStorage.setItem("admin_token", token);
      navigate("/admin");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    }
  };

  return (
    <div style={{ maxWidth: "400px", margin: "4rem auto" }}>
      <h2 style={{ marginBottom: "1.5rem", textAlign: "center" }}>Admin Login</h2>
      <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
        <input
          type="text"
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          required
        />
        <input
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
        />
        {error && <p style={{ color: "#e53e3e", fontSize: "0.85rem" }}>{error}</p>}
        <button type="submit" className="btn btn-primary">
          Login
        </button>
      </form>
    </div>
  );
}
