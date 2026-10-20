//! Petunia3D core neutro: tipos compartilhados, eventos e contrato de módulo.
//! Features conhecem estas abstrações — nunca umas às outras (§22).

pub mod align_commands;
pub use align_commands::{AlignObjectsCmd, DistributeObjectsCmd};
pub mod animate_session;
pub mod brush;
pub mod bvh;
pub mod camera;
pub mod clock;
pub mod command;
pub mod context;
mod context_api;
pub mod cutting_session;
pub mod diagnostics;
pub mod docs;
pub mod edit_api;
pub mod events;
pub mod handles;
pub mod inference;
pub mod jobs;
pub mod loop_cut;
pub mod material_commands;
pub mod measure_commands;
pub mod mesh_preview;
pub mod modal;
pub mod modal_feedback;
pub mod modeling_feedback;
pub mod modifier_commands;
pub mod module;
pub mod object_metadata_commands;
pub mod op_graph_commands;
pub mod origin_commands;
pub mod palette_commands;
pub mod transform_lock_commands;
pub use object_metadata_commands::SetAssetSelectionColorCmd;
pub mod picking;
pub mod poly_pen;
pub mod primitive_session;
pub mod project_service;
pub mod proportional;
pub mod queries;
pub mod recent_projects;
pub mod reference_commands;
pub use reference_commands::{EditReferenceCmd, ReferenceOp};
pub mod region_push;
pub mod render_revision;
pub mod rig_commands;
pub mod schema_contracts;
pub mod section_analysis;
pub use section_analysis::{
    ClearSectionPlaneCmd, FlipSectionPlaneCmd, LoopKind, OffsetSectionPlaneCmd, OrganicCenterline,
    ProfileStation, SectionAnchor, SectionContour, SectionInspector, SectionPlane, SectionResult,
    SetSectionPlaneCmd, StationBranch, SyntheticCap, ThicknessProbeQuery, ThicknessProbeResult,
    WallType,
};
pub mod selection;
pub mod shape_builder;
pub mod snap;
pub mod spatial_query;
pub mod state;
pub mod tool_session;
pub mod transform_projection;
pub mod uv_selection;
pub mod uv_transform_session;
pub use uv_transform_session::{UvTransformKind, UvTransformSession};
pub mod viewport;
pub mod viewport_query;
pub mod visual_assistance;

pub use visual_assistance::{
    ClayMode, ClaySettings, CompanionViewTrigger, GizmoAccessibilityFeedback, LightingAnchor,
    VisualAssistanceProfile, evaluate_gizmo_accessibility,
};

pub use animate_session::{
    AnimatePreview, AnimateSession, BoneDraft, IkDraft, MotionCatalogEntry, MotionUnavailable,
    PosePreview, PreviewBone, motion_catalog, pick_bone,
};
pub use cutting_session::CutSession;
pub use diagnostics::{DiagnosticCategory, DiagnosticEvent, log_event};
pub use docs::DocsTopic;
pub use handles::{Handle, HandleTable};
pub use jobs::{JobChannel, JobsError, par_finite_sum, par_validate};
pub use petunia_project::{
    ArcLengthTable, AssetSummary, AutosaveConfig, AutosaveService, AutosaveStorage,
    FileAutosaveStorage, MAX_GENERATED_VERTICES, ModelLibraryQuery, ModelLibraryService,
    ModelLibrarySort, PathGenerator, PathGeneratorDependencies, PathGeneratorDiagnostics,
    PathGeneratorError, PathGeneratorEvaluation, PathGeneratorEvaluationCache, PathGeneratorKind,
    PathGeneratorQuality, PathGeneratorWarning, ProfileError, ProfileResource, ProfileWorkplane,
    RecoveryCandidate, RecoveryInfo, SessionLockInfo, SplineError, SplineEvaluationCache,
    SplineFrame, SplineHandleMode, SplineInterpolation, SplinePoint, SplineResource, SplineSample,
    SplineSnapSettings, SurfaceAttachment, SurfaceAttachmentError, SurfaceAttachmentStatus,
    SurfaceFrame, SurfaceHit, SurfaceTriangleHandle, SweepGeneratorParameters,
    detach_surface_attachment_keep_world, evaluate_surface_attachment, project_ray_to_surface,
    project_ray_to_surface_target, reproject_surface_attachment,
    reproject_surface_attachment_to_target, slide_surface_attachment, snap_spline_position,
    surface_attachment_status,
};
pub use queries::{
    SceneHierarchyDto, SceneObjectDto, SelectionDetailsDto, ToolStatusDto, UvDiagnosticsDto,
    query_selection_details,
};
pub use recent_projects::{RecentProjectEntry, RecentProjects};
pub use render_revision::{FingerprintFlags, SceneFingerprint, fingerprint_scene};
pub use schema_contracts::{
    CONTRACT_SCHEMA_VERSION, CommandIntent, SceneItemContract, command_intent_schema,
    scene_item_schema,
};

pub use command::{
    AddPrimitiveCmd, AddSplinePointCmd, AttachSplinePointCmd, BakeDecalCmd, BakePathGeneratorCmd,
    BevelCmd, BoxSelectCmd, ClearAllTransformsCmd, ClearLocationCmd, ClearRotationCmd,
    ClearScaleCmd, ClearSelectionCmd, ClearSharpCmd, Command, CommandCategory, CommandContext,
    CommandDispatcher, CommandError, CommandExecutionReport, CommandMetadata, CommandPaletteItem,
    ConnectLoopsCmd, ConvertSplineToPolylineCmd, CreatePathGeneratorCmd, CreateProfileCmd,
    CreateSplineCmd, CycleSelectionDomainCmd, DeleteAssetCmd, DeleteOrDissolveSelectionCmd,
    DeletePathGeneratorCmd, DeleteProfileCmd, DeleteSelectionCmd, DeleteSplineCmd,
    DeleteSplinePointCmd, DetachSplinePointCmd, DissolveCmd, DuplicateAssetCmd,
    DuplicateSelectionCmd, ExportGlbCmd, ExportObjCmd, ExtrudeIndividualCmd, ExtrudeSelectedCmd,
    FlipDiagonalCmd, FlipNormalsCmd, FrameSelectionCmd, ImportGltfCmd, ImportObjCmd, InsetFacesCmd,
    InstantiateAssetCmd, InvertSelectionCmd, JoinAssetIntoActiveCmd, JoinObjectsCmd, KnifeToolCmd,
    LoopCutCmd, MakeFaceCmd, MarkSharpCmd, MergeCenterCmd, MirrorMeshCmd, MoveSplinePointCmd,
    NewProjectCmd, OpenProjectCmd, OrientConsistentlyCmd, OrientOutwardCmd, PrimitiveKind,
    PushPullApplyCmd, PushPullToolCmd, RedoCmd, ReorderAssetCmd, ReprojectSplinePointAttachmentCmd,
    ResetCameraCmd, ReverseSplineCmd, RevolveCmd, RotateSplinePointAttachmentCmd,
    SaveActiveAsAssetCmd, SaveProjectAsCmd, SaveProjectCmd, ScaleSelectionCmd, SelectAllCmd,
    SelectLinkedCmd, SeparateSelectionCmd, SetAssetCollectionCmd, SetAssetTransformCmd,
    SetDecalTransformCmd, SetObjectTransformCmd, SetSelectionDomainCmd, SetShadeSmoothCmd,
    SetSplineClosedCmd, SetSplineHandlesCmd, SlicePlaneCmd, SlideSplinePointAttachmentCmd,
    SmoothByAngleCmd, SubdivideSelectionCmd, SymmetrizeCmd, ToggleCollectionLockCmd,
    ToggleCollectionVisibilityCmd, ToggleCommandPaletteCmd, ToggleHelpCmd, ToggleLockAssetCmd,
    ToggleProjectionCmd, ToggleSettingsCmd, ToggleVisibilityAssetCmd, ToggleWireframeCmd,
    ToggleXRayCmd, TranslateSelectionCmd, TriangulateMeshCmd, UndoCmd, UnwrapAutoCmd,
    UpdateProfileCmd, UpdateSplineCmd, UpdateSweepGeneratorCmd, UvEqualizeTexelDensityCmd,
    UvNormalizeTexelDensityCmd, UvPackIslandsCmd, UvProjectCubeCmd, UvProjectCylindricalCmd,
    UvProjectFromViewCmd, UvProjectPlanarCmd, UvProjectSphericalCmd, UvSetTexelDensityCmd,
    UvSplitCmd, UvTransformCmd, WeldCmd,
};
pub use context::{
    CommandEffects, EditorView, HostRequest, KeyText, TextSource, TextureOp, UiRequest,
};
pub use project_service::{
    ProjectService, ProjectServiceError, RecoverySummary, sanitize_filename,
};
pub use rig_commands::{
    AddAnimationCmd, AddBoneCmd, AddIkChainCmd, AddMotionCmd, AddMotionLayerCmd, AddRigPresetCmd,
    AddStructuralJointCmd, AddStructuralPartCmd, ApplyIkPoseCmd, ApplyMotionNowCmd,
    ApplyPoseAsRestCmd, AssignRigRoleCmd, AutoRigActiveAssetCmd, BindRigCmd, ClearBoneTrackCmd,
    ClearRigRoleCmd, ClearVertexWeightOverrideCmd, ConnectStructuralPartsCmd,
    CreateEmptySkeletonCmd, CreateStructuralGraphCmd, DeleteBoneKeyCmd, DeleteBoneKeyframeCmd,
    DuplicateMotionCmd, EditBoneRestPoseCmd, FitBlocked, FitRigToActiveAssetCmd, InferRigRolesCmd,
    InsertBoneKeyframeCmd, RemapRigWeightsCmd, RemoveAnimationCmd, RemoveBoneCmd, RemoveIkChainCmd,
    RemoveMotionCmd, RemoveMotionLayerCmd, RemoveSkeletonCmd, RemoveStructuralPartCmd,
    RenameBoneCmd, ReparentBoneCmd, ResetAllBonePosesCmd, ResetBonePoseCmd, RigPresetKind,
    SetBoneKeyCmd, SetBoneKeyframeInterpolationCmd, SetBoneTransformCmd, SetLiveAutoUpdateCmd,
    SetMotionLayerBlendModeCmd, SetMotionLayerWeightCmd, SetMotionParamCmd, SetMotionStyleCmd,
    SetSkinningEnabledCmd, SetStructuralBoneLockedCmd, SetStructuralPartLockedCmd,
    SetVertexWeightOverrideCmd, SkinningPreviewState, ToggleMotionLayerEnabledCmd, UnbindRigCmd,
    UpdateIkChainCmd, UpdateMotionCmd, UpdateStructuralRigCmd, fit_availability,
};

pub use brush::{
    BRUSH_PX_PER_UNIT, BrushBlend, BrushLock, BrushPreset, BrushPreviewKind, BrushPreviewStyle,
    BrushProjectionMode, BrushSettings, BrushStyle, BrushTip, BrushType, FillScope,
    PaintBlockedReason, PaintRestriction, PointStabilizer, StrokeBuffer, StrokeSampler,
    brush_size_px_from_slider, brush_type_from_kind, hash01, kind_from_brush_type,
};
pub use camera::{Camera, NavigationAnchor, Projection, ViewPreset};
pub use clock::{Clock, FixedClock, SystemClock};
pub use edit_api::{
    AddMeshAssetCmd, ApplyMeshEditCmd, AssignMaterialSlotCmd, ComponentSet, InsertAssetCmd,
    NudgeCmd, PinOp, SeamOp, SelectionOp, UvPinCmd, UvSeamCmd,
};
pub use events::{AppEvent, EventBus};
pub use inference::{
    DEFAULT_SNAP_RADIUS_PIXELS, ScreenSnapHit, ScreenSnapQuery, SnapAccel, SnapAnchor, SnapGrid,
    SnapKind, SnapMask, SnapSource, clamp_snap_radius, snap_screen, snap_screen_with,
};
pub use material_commands::{
    CreateMaterialCmd, DuplicateMaterialCmd, EditMaterialCmd, MaterialEdit, MaterialRecipeCmd,
    RecipeOp, RemoveMaterialCmd, SetMaterialPropertiesCmd,
};
pub use modal_feedback::ToolFeedback;
pub use modeling_feedback::{
    ActiveDimension, FeedbackMarker, FeedbackWarning, MeasurementQuality,
    ModelingFeedbackInspector, OperationFeedback, PartialSelectionReport, WarningSeverity,
};
pub use modifier_commands::{ModifierCmd, ModifierOp};
pub use module::{Module, ModuleRegistry};
pub use op_graph_commands::{GraphOp, OpGraphCmd};
pub use petunia_mesh::poly_pen::PenPoint;
pub use poly_pen::{PenStrip, PolyPenCommandError};
pub use primitive_session::{
    CircleFill, PrimitiveCreationSession, PrimitiveDescriptor, PrimitiveDescriptorExt,
};
pub use proportional::{ProportionalFalloff, ProportionalSettings, calculate_falloff_weight};
pub use region_push::{RegionHit, RegionPlane, RegionPlanes, RegionPushError, region_at_ray};
pub use selection::{
    ActiveElement, EdgeKey, FaceIndex, ResolvedTransformTargets, SelectMode, Selection,
    SelectionDomain, SelectionFilters, SelectionRef, SelectionResolver, SelectionState,
    VertexIndex, Workspace,
};
pub use shape_builder::{
    PathfinderOp, ShapeEdit, ShapeEditCmd, ShapeEditError, ShapeEditReport, ShapePlane,
};
pub mod scene_commands;
pub mod solid_commands;
pub use measure_commands::{AddMeasurementCmd, ClearMeasurementsCmd, MIN_MEASURE_DISTANCE};
pub use palette_commands::{PaletteCmd, PaletteOp};
pub use scene_commands::{
    GroupSelectionCmd, MoveAssetsCmd, RenameGroupCmd, SetGroupColorCmd, UngroupCmd,
};
pub use snap::{
    SnapElement, SnapQuery, SnapResult, SnapSettings, SnapTarget, snap_point, snap_point_to_edges,
    snap_point_to_faces, snap_point_to_grid, snap_point_to_increment, snap_point_to_vertices,
};
pub use solid_commands::{
    BakeSolidCompositionCmd, CreateSolidCompositionCmd, ReorderSolidCompositionStepCmd,
    SolidOperation, SolidShapeBuilderReport, SolidShapeEditCmd, ToggleSolidCompositionStepCmd,
};
pub use spatial_query::{
    CompanionViewState, GeometryRepresentation, ScaleToDimensionCmd, SpatialFrame,
    SpatialFrameKind, SpatialQueryInspector, SpatialQueryKind, SpatialQueryResult, VolumeFrame,
};
pub use transform_lock_commands::SetTransformLockCmd;
pub mod surface_commands;
pub use state::{
    ASSET_NAME_MAX_LEN, ActiveTool, ActiveToolSessionKind, AnnotationItem, AnnotationStroke,
    AppState, AssetRenameError, DirtyReason, DockOrientation, DockSide, Document, DocumentHistory,
    DocumentPath, DocumentSession, DomainState, EditMode, EditPivotSession, EditorSession,
    GridSettings, HoverTarget, Measurement, MeasurementItem, PROPERTIES_DEFAULT_WIDTH,
    PROPERTIES_MAX_WIDTH, PROPERTIES_MIN_WIDTH, PivotPoint, ProfileState, ProjectState, RefAxis,
    ReferenceImage, RenderResources, RenderStats, SHELL_ASSET_LIBRARY_DEFAULT_HEIGHT,
    SHELL_ASSET_LIBRARY_MAX_HEIGHT, SHELL_ASSET_LIBRARY_MIN_HEIGHT, SceneFilter, SceneObjectState,
    SelectPolicy, Shading, TOOLBAR_DEFAULT_WIDTH, TextureDirtyRect, TextureUpdate, ToolActivation,
    ToolSessionManager, ToolSettingsSnapshot, ToolState, TransformFrame, TransformOrientation,
    UiDensity, UiState, ViewportSession, WorkplaneKind, WorkspaceUiMemory, workspace_index,
};
pub use surface_commands::{
    AlignWorkplaneToFaceCmd, AlignWorkplaneToSelectionCmd, ContextualPushPullCmd, ImprintProfileCmd,
};
pub use uv_selection::{UvSelectionMode, UvSelectionState};
pub use viewport::{
    LogicalRect, PhysicalViewport, unproject_cursor_or_vertex_snap,
    unproject_to_surface_or_cursor_plane,
};
pub use viewport_query::{ViewportQueryBuffer, ViewportQuerySample};

pub type AttachmentValidity = SurfaceAttachmentStatus;

pub use modal::{ModalConstraint, ModalError, ModalKind, ModalOp};
pub use tool_session::{
    DEFAULT_DRAG_THRESHOLD_PX, DRAG_THRESHOLD_RANGE, DragFrame, LastOperation, PressTarget,
    ToolEffect, ToolKey, ToolPhase, ToolSession, drag_value,
};

/// Malha, reexportada para os shells que manipulam geometria sem depender de `petunia_mesh`.
pub use petunia_mesh::Mesh;
/// Ponto de corte de aresta da faca, reexportado para os shells não dependerem
/// de `petunia_mesh` diretamente.
pub use petunia_mesh::knife::EdgePoint as CutEdgePoint;
/// Anel de faces/arestas do loop cut, reexportado pelo mesmo motivo.
pub use petunia_mesh::loop_cut::{LoopCutError, LoopRing};

#[cfg(test)]
mod preview_tests;

mod draw_batch;
pub use draw_batch::DrawBatchCmd;

mod boolean_texture;

pub mod geometry_generators;
pub use geometry_generators::{
    CreateGeometryGeneratorCmd, MakeGeneratedEditableCmd, UpdateGeometryGeneratorCmd,
};

pub mod workplane;
pub use workplane::{WorkplaneMode, WorkplaneSession};

pub use geometry_generators::CreateRegionGeneratorCmd;

pub use geometry_generators::EditPlanarShapeCmd;

pub mod solid_shape_builder;
pub use solid_shape_builder::{
    CommitSolidShapeBuilderCmd, SolidShapeBuilderSession, SolidShapeIntent,
};

pub use geometry_generators::SetPlanarShapeFlagsCmd;
