% tiyatro.cls — a XeLaTeX class for stage plays, part of Dactile.
% https://github.com/D4shwood/dactile
%
% Copyright (C) 2026 D4shwood
% This program is free software: you can redistribute it and/or modify it under
% the terms of the GNU General Public License as published by the Free Software
% Foundation, either version 3 of the License, or (at your option) any later
% version. See LICENSE.
%
%   \documentclass[draft]{tiyatro}   line numbers, 1.5 spacing, "TASLAK" on the cover
%   \documentclass[final]{tiyatro}   clean copy for print
%
% The class is split in two: tiyatro-core.sty (characters, lines, scenes, acts,
% the cast table) and tiyatro-theme.sty (page, fonts, words, cover). A different
% look only needs a different theme.

\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{tiyatro}[2026/10/25 v1.0.0 Dactile stage play class]

\newif\iftiyatrodraft
\tiyatrodrafttrue
\DeclareOption{draft}{\tiyatrodrafttrue}
\DeclareOption{final}{\tiyatrodraftfalse}
\DeclareOption*{\PassOptionsToClass{\CurrentOption}{book}}
\ProcessOptions\relax

% Latin Modern, LaTeX's default font, is missing from some TeX installs, and
% the base class and fontspec ask for it before the theme picks the real fonts.
% Where it is missing, those requests get Liberation Serif.
\IfFileExists{lmroman12-regular.otf}{}{%
  \DeclareFontFamily{TU}{lmr}{}%
  \DeclareFontShape{TU}{lmr}{m}{n}{<->"[LiberationSerif-Regular.ttf]"}{}%
  \DeclareFontShape{TU}{lmr}{m}{it}{<->"[LiberationSerif-Italic.ttf]"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{n}{<->"[LiberationSerif-Bold.ttf]"}{}%
  \DeclareFontShape{TU}{lmr}{bx}{it}{<->"[LiberationSerif-BoldItalic.ttf]"}{}%
  \DeclareFontShape{TU}{lmr}{b}{n}{<->ssub * lmr/bx/n}{}%
  \DeclareFontShape{TU}{lmr}{m}{sl}{<->ssub * lmr/m/it}{}%
  \DeclareFontShape{TU}{lmr}{m}{sc}{<->ssub * lmr/m/n}{}%
}

\LoadClass[12pt,a4paper,oneside]{book}

% The theme first: names are measured in the font they are printed in.
\RequirePackage{tiyatro-theme}
\RequirePackage{tiyatro-core}
