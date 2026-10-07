namespace Moth.PokeIdle.IdleShell;

/// <summary>
/// Account manager dialog: one row per managed login (PokéIdle / Twitch / Kick).
/// Add, rename, switch service, enable/disable (route links or not), delete, and
/// "Log in…" which shows that account's stream slot so the sign-in can be completed
/// interactively.
/// </summary>
internal sealed class AccountsDialog : Form
{
    private static readonly (string Name, AccountService Service)[] Services =
    [
        ("PokéIdle", AccountService.PokeIdle),
        ("Twitch", AccountService.Twitch),
        ("Kick", AccountService.Kick),
    ];

    private readonly AccountManager _accounts;
    private readonly Func<Account, bool> _isActive;
    private readonly Action<Account> _logIn;
    private readonly ListView _list = new()
    {
        Dock = DockStyle.Fill,
        View = View.Details,
        FullRowSelect = true,
        MultiSelect = false,
        HideSelection = false
    };
    private readonly ComboBox _serviceCombo = new()
    {
        DropDownStyle = ComboBoxStyle.DropDownList,
        Width = 100
    };

    private bool _refreshing;

    public AccountsDialog(
        AccountManager accounts,
        Func<Account, bool> isActive,
        Action<Account> logIn)
    {
        _accounts = accounts;
        _isActive = isActive;
        _logIn = logIn;

        Text = "Accounts";
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        ClientSize = new Size(640, 400);
        ShowInTaskbar = false;

        _list.Columns.Add("Profile id", 100);
        _list.Columns.Add("Label", 160);
        _list.Columns.Add("Service", 80);
        _list.Columns.Add("Routing", 70);
        _list.Columns.Add("State", 70);
        _list.Columns.Add("Capacity", 75);
        _list.DoubleClick += (_, _) => LogInSelected();
        _list.SelectedIndexChanged += (_, _) => SyncServiceCombo();

        foreach (var (name, _) in Services) _serviceCombo.Items.Add(name);
        _serviceCombo.Location = new Point(8, 8);
        _serviceCombo.SelectedIndexChanged += (_, _) =>
        {
            if (_refreshing) return;
            var acc = Selected();
            if (acc is null) return;
            var svc = Services[_serviceCombo.SelectedIndex].Service;
            if (svc == acc.Service) return;
            try { _accounts.SetService(acc, svc); }
            catch (Exception ex)
            {
                MessageBox.Show(this, ex.Message, "Accounts", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            }
            RefreshList();
        };

        var buttons = new FlowLayoutPanel
        {
            Dock = DockStyle.Bottom,
            Height = 40,
            FlowDirection = FlowDirection.LeftToRight,
            Padding = new Padding(6, 6, 6, 4)
        };
        Button("Add selected service", () => AddAccount(Services[Math.Max(0, _serviceCombo.SelectedIndex)].Service));
        Button("Rename", RenameSelected);
        Button("Toggle routing", ToggleRoutingSelected);
        Button("Delete", DeleteSelected);
        Button("Log in…", LogInSelected);

        Controls.Add(_list);
        Controls.Add(_serviceCombo);
        Controls.Add(buttons);

        RefreshList();
        return;

        void Button(string text, Action act)
        {
            var b = new Button { Text = text, AutoSize = true, Height = 28 };
            b.Click += (_, _) => act();
            buttons.Controls.Add(b);
        }
    }

    private void AddAccount(AccountService service)
    {
        var label = PromptText($"New {service} account label",
            service == AccountService.PokeIdle ? "Account C" : "login name");
        if (label is null) return;
        try
        {
            _accounts.Add(label, service);
            RefreshList();
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, ex.Message, "Accounts", MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
    }

    private Account? Selected() =>
        _accounts.Find(_list.SelectedItems.Count > 0
            ? _list.SelectedItems[0].Text
            : string.Empty);

    private void SyncServiceCombo()
    {
        var acc = Selected();
        _refreshing = true;
        try
        {
            var idx = acc is null ? -1 : Array.FindIndex(Services, s => s.Service == acc.Service);
            _serviceCombo.SelectedIndex = idx;
        }
        finally { _refreshing = false; }
    }

    private void RenameSelected()
    {
        var acc = Selected();
        if (acc is null) return;
        var label = PromptText("Rename account", acc.Label);
        if (label is null) return;
        try { _accounts.Rename(acc, label); }
        catch (Exception ex)
        {
            MessageBox.Show(this, ex.Message, "Accounts", MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
        RefreshList();
    }

    private void ToggleRoutingSelected()
    {
        var acc = Selected();
        if (acc is null || !acc.IsStream) return;
        _accounts.SetEnabled(acc, !acc.Enabled);
        RefreshList();
    }

    private void DeleteSelected()
    {
        var acc = Selected();
        if (acc is null) return;
        if (acc.Service == AccountService.PokeIdle &&
            _accounts.GameAccounts.Count() <= 1)
        {
            MessageBox.Show(this, "At least one game account is required.",
                "Accounts", MessageBoxButtons.OK, MessageBoxIcon.Information);
            return;
        }
        var confirm = MessageBox.Show(this,
            $"Remove account \"{acc.Label}\" ({acc.Id})?\n\n" +
            "Its saved login/session data stays on disk and can be reused " +
            "if you add an account with the same id.",
            "Accounts", MessageBoxButtons.YesNo, MessageBoxIcon.Question);
        if (confirm != DialogResult.Yes) return;
        _accounts.Remove(acc);
        RefreshList();
    }

    private void LogInSelected()
    {
        var acc = Selected();
        if (acc is null) return;
        _logIn(acc);
        Close();
    }

    private void RefreshList()
    {
        var selectedId = Selected()?.Id;
        _list.BeginUpdate();
        _list.Items.Clear();
        foreach (var a in _accounts.All)
        {
            var item = new ListViewItem(a.Id);
            item.SubItems.Add(a.Label);
            item.SubItems.Add(a.Service.ToString());
            item.SubItems.Add(!a.IsStream ? "—" : a.Enabled ? "On" : "Off");
            item.SubItems.Add(!a.IsStream
                ? "game"
                : _isActive(a) ? "connected" : "offline");
            item.SubItems.Add(a.IsStream
                ? $"0–{AccountManager.MaxStreamSlotsPerAccount} chats"
                : "—");
            item.Tag = a;
            if (string.Equals(a.Id, selectedId, StringComparison.OrdinalIgnoreCase))
                item.Selected = true;
            _list.Items.Add(item);
        }
        _list.EndUpdate();
        if (_list.SelectedItems.Count == 0 && _list.Items.Count > 0)
            _list.Items[0].Selected = true;
        SyncServiceCombo();
    }

    private string? PromptText(string title, string initial)
    {
        using var form = new Form
        {
            Text = title,
            FormBorderStyle = FormBorderStyle.FixedDialog,
            StartPosition = FormStartPosition.CenterParent,
            ClientSize = new Size(380, 74),
            MaximizeBox = false,
            MinimizeBox = false
        };
        var box = new TextBox { Left = 10, Top = 12, Width = 360, Text = initial };
        var ok = new Button { Text = "OK", Left = 210, Top = 42, DialogResult = DialogResult.OK };
        var cancel = new Button { Text = "Cancel", Left = 295, Top = 42, DialogResult = DialogResult.Cancel };
        form.Controls.AddRange([box, ok, cancel]);
        form.AcceptButton = ok;
        form.CancelButton = cancel;
        box.SelectAll();
        box.Focus();
        return form.ShowDialog(this) == DialogResult.OK ? box.Text.Trim() : null;
    }
}
