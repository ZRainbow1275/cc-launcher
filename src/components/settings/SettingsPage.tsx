import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  Loader2,
  Save,
  FolderSearch,
  Database,
  Cloud,
  ScrollText,
  HardDriveDownload,
  FlaskConical,
  Info,
} from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { settingsApi } from "@/lib/api";
import { GeneralSettingsD4Section } from "@/components/settings/GeneralSettingsD4Section";
import { LanguageSettings } from "@/components/settings/LanguageSettings";
import { ThemeSettings } from "@/components/settings/ThemeSettings";
import { WindowSettings } from "@/components/settings/WindowSettings";
import { AppVisibilitySettings } from "@/components/settings/AppVisibilitySettings";
import { SkillStorageLocationSettings } from "@/components/settings/SkillStorageLocationSettings";
import { SkillSyncMethodSettings } from "@/components/settings/SkillSyncMethodSettings";
import { TerminalSettings } from "@/components/settings/TerminalSettings";
import { DirectorySettings } from "@/components/settings/DirectorySettings";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import { BackupListSection } from "@/components/settings/BackupListSection";
import { WebdavSyncSection } from "@/components/settings/WebdavSyncSection";
import { AboutSection } from "@/components/settings/AboutSection";
import { ProxyTabContent } from "@/components/settings/ProxyTabContent";
import { ModelTestConfigPanel } from "@/components/usage/ModelTestConfigPanel";
import { UsageDashboard } from "@/components/usage/UsageDashboard";
import { LogConfigPanel } from "@/components/settings/LogConfigPanel";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";
import { useInstalledSkills } from "@/hooks/useSkills";
import { useSettings } from "@/hooks/useSettings";
import { useImportExport } from "@/hooks/useImportExport";
import { useTranslation } from "react-i18next";
import type { SettingsFormState } from "@/hooks/useSettings";

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
}

/**
 * Fallback placeholder shown when the underlying cc-switch settings IPC
 * (`get_settings`, `is_portable_mode`, OAuth account list, …) is unavailable —
 * i.e. running under `VITE_MOCK_IPC=1` or before the Rust backend has spun up.
 * Renders an informative empty-state card per tab so the tab never appears
 * blank to the user.
 */
interface SettingsTabFallbackProps {
  title: string;
  description: string;
  footerNote: string;
}

function SettingsTabFallback({
  title,
  description,
  footerNote,
}: SettingsTabFallbackProps) {
  return (
    <Card data-testid="settings-tab-fallback" className="glass-card">
      <CardHeader>
        <div className="flex items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-muted">
            <Info className="h-4 w-4 text-muted-foreground" />
          </div>
          <div>
            <CardTitle className="text-base">{title}</CardTitle>
            <CardDescription className="text-sm">{description}</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-xs text-muted-foreground">{footerNote}</p>
      </CardContent>
    </Card>
  );
}

export function SettingsPage({
  open,
  onOpenChange,
  onImportSuccess,
  defaultTab = "general",
}: SettingsDialogProps) {
  const { t } = useTranslation();
  const {
    settings,
    isLoading,
    isSaving,
    isPortable,
    appConfigDir,
    resolvedDirs,
    updateSettings,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    saveSettings,
    autoSaveSettings,
    requiresRestart,
    acknowledgeRestart,
  } = useSettings();

  const {
    selectedFile,
    status: importStatus,
    errorMessage,
    backupId,
    isImporting,
    selectImportFile,
    importConfig,
    exportConfig,
    clearSelection,
    resetStatus,
  } = useImportExport({ onImportSuccess });

  const { data: installedSkills } = useInstalledSkills();

  const [activeTab, setActiveTab] = useState<string>("general");
  const [showRestartPrompt, setShowRestartPrompt] = useState(false);

  useEffect(() => {
    if (open) {
      setActiveTab(defaultTab);
      resetStatus();
    }
  }, [open, resetStatus, defaultTab]);

  useEffect(() => {
    if (requiresRestart) {
      setShowRestartPrompt(true);
    }
  }, [requiresRestart]);

  const closeAfterSave = useCallback(() => {
    // 保存成功后关闭：不再重置语言，避免需要“保存两次”才生效
    acknowledgeRestart();
    clearSelection();
    resetStatus();
    onOpenChange(false);
  }, [acknowledgeRestart, clearSelection, onOpenChange, resetStatus]);

  const handleSave = useCallback(async () => {
    try {
      const result = await saveSettings(undefined, { silent: false });
      if (!result) return;
      if (result.requiresRestart) {
        setShowRestartPrompt(true);
        return;
      }
      closeAfterSave();
    } catch (error) {
      console.error("[SettingsPage] Failed to save settings", error);
    }
  }, [closeAfterSave, saveSettings]);

  const handleRestartLater = useCallback(() => {
    setShowRestartPrompt(false);
    closeAfterSave();
  }, [closeAfterSave]);

  const handleRestartNow = useCallback(async () => {
    setShowRestartPrompt(false);
    if (import.meta.env.DEV) {
      toast.success(t("settings.devModeRestartHint"), { closeButton: true });
      closeAfterSave();
      return;
    }

    try {
      await settingsApi.restart();
    } catch (error) {
      console.error("[SettingsPage] Failed to restart app", error);
      toast.error(t("settings.restartFailed"));
    } finally {
      closeAfterSave();
    }
  }, [closeAfterSave, t]);

  // 通用设置即时保存（无需手动点击）
  // 使用 autoSaveSettings 避免误触发系统 API（开机自启、Claude 插件等）
  const handleAutoSave = useCallback(
    async (updates: Partial<SettingsFormState>) => {
      if (!settings) return;
      updateSettings(updates);
      try {
        await autoSaveSettings(updates);
      } catch (error) {
        console.error("[SettingsPage] Failed to autosave settings", error);
        toast.error(
          t("settings.saveFailedGeneric", {
            defaultValue: "保存失败，请重试",
          }),
        );
      }
    },
    [autoSaveSettings, settings, t, updateSettings],
  );

  const isBusy = useMemo(() => isLoading && !settings, [isLoading, settings]);

  return (
    <div className="flex flex-col h-full overflow-hidden px-6">
      {isBusy ? (
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="flex flex-col h-full"
        >
          <TabsList className="grid w-full grid-cols-6 mb-6 glass rounded-lg">
            <TabsTrigger value="general">
              {t("settings.tabGeneral")}
            </TabsTrigger>
            <TabsTrigger value="proxy">{t("settings.tabProxy")}</TabsTrigger>
            <TabsTrigger value="auth">
              {t("settings.tabAuth", { defaultValue: "认证" })}
            </TabsTrigger>
            <TabsTrigger value="advanced">
              {t("settings.tabAdvanced")}
            </TabsTrigger>
            <TabsTrigger value="usage">{t("usage.title")}</TabsTrigger>
            <TabsTrigger value="about">{t("common.about")}</TabsTrigger>
          </TabsList>

          <div className="flex-1 min-h-0 flex flex-col">
            <div className="flex-1 overflow-y-auto overflow-x-hidden pr-2">
              <TabsContent value="general" className="space-y-6 mt-0">
                <GeneralSettingsD4Section />
                {settings ? (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className="space-y-6"
                  >
                    <LanguageSettings
                      value={settings.language}
                      onChange={(lang) => handleAutoSave({ language: lang })}
                    />
                    <ThemeSettings />
                    <AppVisibilitySettings
                      settings={settings}
                      onChange={handleAutoSave}
                    />
                    <SkillStorageLocationSettings
                      value={settings.skillStorageLocation ?? "cc_switch"}
                      installedCount={installedSkills?.length ?? 0}
                      onMigrated={(location) =>
                        updateSettings({ skillStorageLocation: location })
                      }
                    />
                    <SkillSyncMethodSettings
                      value={settings.skillSyncMethod ?? "auto"}
                      onChange={(method) =>
                        handleAutoSave({ skillSyncMethod: method })
                      }
                    />
                    <WindowSettings
                      settings={settings}
                      onChange={handleAutoSave}
                    />
                    <TerminalSettings
                      value={settings.preferredTerminal}
                      onChange={(terminal) =>
                        handleAutoSave({ preferredTerminal: terminal })
                      }
                    />
                  </motion.div>
                ) : (
                  <SettingsTabFallback
                    title={t("settings.fallback.general.title", {
                      defaultValue: "通用配置",
                    })}
                    description={t("settings.fallback.general.description", {
                      defaultValue:
                        "管理语言 / 主题 / 应用可见性 / 技能存储位置 / 窗口与终端等基础偏好。",
                    })}
                    footerNote={t("settings.fallback.note", {
                      defaultValue:
                        "等待与后端 IPC 接通后可编辑（当前运行于 Mock 模式）",
                    })}
                  />
                )}
              </TabsContent>

              <TabsContent value="proxy" className="space-y-6 mt-0 pb-4">
                {settings ? (
                  <ProxyTabContent
                    settings={settings}
                    onAutoSave={handleAutoSave}
                  />
                ) : (
                  <SettingsTabFallback
                    title={t("settings.fallback.proxy.title", {
                      defaultValue: "路由配置",
                    })}
                    description={t("settings.fallback.proxy.description", {
                      defaultValue:
                        "管理本地路由总开关、各应用代理接管、故障转移及全局代理。",
                    })}
                    footerNote={t("settings.fallback.note", {
                      defaultValue:
                        "等待与后端 IPC 接通后可编辑（当前运行于 Mock 模式）",
                    })}
                  />
                )}
              </TabsContent>

              <TabsContent value="auth" className="space-y-6 mt-0 pb-4">
                {settings ? (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className="space-y-6"
                  >
                    <AuthCenterPanel />
                  </motion.div>
                ) : (
                  <SettingsTabFallback
                    title={t("settings.fallback.auth.title", {
                      defaultValue: "认证中心",
                    })}
                    description={t("settings.fallback.auth.description", {
                      defaultValue:
                        "管理 GitHub Copilot 与 ChatGPT (Codex OAuth) 账号登录、刷新与切换。",
                    })}
                    footerNote={t("settings.fallback.note", {
                      defaultValue:
                        "等待与后端 IPC 接通后可编辑（当前运行于 Mock 模式）",
                    })}
                  />
                )}
              </TabsContent>

              <TabsContent value="advanced" className="space-y-6 mt-0 pb-4">
                {settings ? (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className="space-y-4"
                  >
                    <Accordion
                      type="multiple"
                      defaultValue={[]}
                      className="w-full space-y-4"
                    >
                      <AccordionItem
                        value="directory"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <FolderSearch className="h-5 w-5 text-primary" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.configDir.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.configDir.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <DirectorySettings
                            appConfigDir={appConfigDir}
                            resolvedDirs={resolvedDirs}
                            onAppConfigChange={updateAppConfigDir}
                            onBrowseAppConfig={browseAppConfigDir}
                            onResetAppConfig={resetAppConfigDir}
                            claudeDir={settings.claudeConfigDir}
                            codexDir={settings.codexConfigDir}
                            geminiDir={settings.geminiConfigDir}
                            opencodeDir={settings.opencodeConfigDir}
                            openclawDir={settings.openclawConfigDir}
                            hermesDir={settings.hermesConfigDir}
                            onDirectoryChange={updateDirectory}
                            onBrowseDirectory={browseDirectory}
                            onResetDirectory={resetDirectory}
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="data"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <Database className="h-5 w-5 text-blue-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.data.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.data.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <ImportExportSection
                            status={importStatus}
                            selectedFile={selectedFile}
                            errorMessage={errorMessage}
                            backupId={backupId}
                            isImporting={isImporting}
                            onSelectFile={selectImportFile}
                            onImport={importConfig}
                            onExport={exportConfig}
                            onClear={clearSelection}
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="backup"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <HardDriveDownload className="h-5 w-5 text-amber-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.backup.title", {
                                  defaultValue: "Backup & Restore",
                                })}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.backup.description", {
                                  defaultValue:
                                    "Manage automatic backups, view and restore database snapshots",
                                })}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <BackupListSection
                            backupIntervalHours={settings.backupIntervalHours}
                            backupRetainCount={settings.backupRetainCount}
                            onSettingsChange={(updates) =>
                              handleAutoSave(updates)
                            }
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="cloudSync"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <Cloud className="h-5 w-5 text-blue-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.cloudSync.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.cloudSync.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <WebdavSyncSection
                            config={settings?.webdavSync}
                            settings={settings}
                            onAutoSave={handleAutoSave}
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="test"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <FlaskConical className="h-5 w-5 text-emerald-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.modelTest.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.modelTest.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <ModelTestConfigPanel />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="logConfig"
                        className="rounded-xl glass-card overflow-hidden"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <ScrollText className="h-5 w-5 text-cyan-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.logConfig.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.logConfig.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <LogConfigPanel />
                        </AccordionContent>
                      </AccordionItem>
                    </Accordion>
                  </motion.div>
                ) : (
                  <SettingsTabFallback
                    title={t("settings.fallback.advanced.title", {
                      defaultValue: "高级",
                    })}
                    description={t("settings.fallback.advanced.description", {
                      defaultValue:
                        "管理配置目录、导入/导出、备份与恢复、WebDAV 云同步、模型测试与日志配置。",
                    })}
                    footerNote={t("settings.fallback.note", {
                      defaultValue:
                        "等待与后端 IPC 接通后可编辑（当前运行于 Mock 模式）",
                    })}
                  />
                )}
              </TabsContent>

              <TabsContent value="about" className="mt-0">
                <AboutSection isPortable={isPortable} />
              </TabsContent>

              <TabsContent value="usage" className="mt-0">
                <UsageDashboard />
              </TabsContent>
            </div>

            {activeTab === "advanced" && settings && (
              <div
                className="flex-shrink-0 pt-4 border-t border-border-default"
                style={{ backgroundColor: "hsl(var(--background))" }}
              >
                <div className="px-6 flex items-center justify-end gap-3">
                  <Button onClick={handleSave} disabled={isSaving}>
                    {isSaving ? (
                      <span className="inline-flex items-center gap-2">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        {t("settings.saving")}
                      </span>
                    ) : (
                      <>
                        <Save className="mr-2 h-4 w-4" />
                        {t("common.save")}
                      </>
                    )}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </Tabs>
      )}

      <Dialog
        open={showRestartPrompt}
        onOpenChange={(open) => !open && handleRestartLater()}
      >
        <DialogContent zIndex="alert" className="max-w-md glass border-border">
          <DialogHeader>
            <DialogTitle>{t("settings.restartRequired")}</DialogTitle>
          </DialogHeader>
          <div className="px-6">
            <p className="text-sm text-muted-foreground">
              {t("settings.restartRequiredMessage")}
            </p>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={handleRestartLater}
              className="hover:bg-muted/50"
            >
              {t("settings.restartLater")}
            </Button>
            <Button
              onClick={handleRestartNow}
              className="bg-primary hover:bg-primary/90"
            >
              {t("settings.restartNow")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
