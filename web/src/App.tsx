import { Routes, Route } from "react-router-dom";
import Layout from "./components/Layout";
import RequireAuth from "./components/RequireAuth";
import Home from "./pages/Home";
import TemplatePage from "./pages/TemplatePage";
import SearchResults from "./pages/SearchResults";
import AuthorPage from "./pages/AuthorPage";
import Login from "./pages/admin/Login";
import Dashboard from "./pages/admin/Dashboard";
import FeaturedManager from "./pages/admin/FeaturedManager";
import BlacklistManager from "./pages/admin/BlacklistManager";
import AdminUsers from "./pages/admin/AdminUsers";

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Home />} />
        <Route path="/templates/:addr" element={<TemplatePage />} />
        <Route path="/search" element={<SearchResults />} />
        <Route path="/authors/:pubkey" element={<AuthorPage />} />
        <Route path="/admin/login" element={<Login />} />
        <Route path="/admin" element={<RequireAuth><Dashboard /></RequireAuth>} />
        <Route path="/admin/featured" element={<RequireAuth><FeaturedManager /></RequireAuth>} />
        <Route path="/admin/blacklist" element={<RequireAuth><BlacklistManager /></RequireAuth>} />
        <Route path="/admin/users" element={<RequireAuth><AdminUsers /></RequireAuth>} />
      </Route>
    </Routes>
  );
}
