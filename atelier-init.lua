-- Load user's original configuration if it exists
local user_init_lua = vim.fn.expand("~/.config/nvim/init.lua")
local user_init_vim = vim.fn.expand("~/.config/nvim/init.vim")
if vim.fn.filereadable(user_init_lua) == 1 then
  vim.cmd("source " .. user_init_lua)
elseif vim.fn.filereadable(user_init_vim) == 1 then
  vim.cmd("source " .. user_init_vim)
end

-- Ensure mapleader is set
vim.g.mapleader = vim.g.mapleader or " "

local function write_to_file(filepath, content)
  local f = io.open(filepath, "w")
  if f then
    f:write(content)
    f:close()
  end
end

_G.atelier_send_line = function()
  local line = vim.api.nvim_get_current_line()
  write_to_file("/tmp/atelier-cmd", line .. "\n")
end

_G.atelier_send_selection = function()
  local _, ls, cs = unpack(vim.fn.getpos("'<"))
  local _, le, ce = unpack(vim.fn.getpos("'>"))
  local lines = vim.api.nvim_buf_get_lines(0, ls - 1, le, false)
  if #lines == 0 then return end
  local content = table.concat(lines, "\n") .. "\n"
  write_to_file("/tmp/atelier-cmd", content)
end

-- Set keymaps
vim.keymap.set('n', '<leader>e', ':lua atelier_send_line()<CR>', { silent = true })
vim.keymap.set('v', '<leader>e', ':lua atelier_send_selection()<CR>', { silent = true })
vim.keymap.set('n', '<C-CR>', ':lua atelier_send_line()<CR>', { silent = true })
vim.keymap.set('v', '<C-CR>', ':lua atelier_send_selection()<CR>', { silent = true })
